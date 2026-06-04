use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::Seek;
use std::io::Write;
use std::path::Path;

use crate::api::KeyValueStore;
use crate::error::Result;
use crate::storage::memtable::MemTable;
use crate::storage::wal;
use crate::storage::wal::ReadRecord;
use crate::storage::wal::WalRecord;
use crate::types::Key;
use crate::types::KeyRange;
use crate::types::KeyValue;
use crate::types::Value;

const WAL_FILE_NAME: &str = "bloomy.wal";

#[derive(Debug)]
pub struct LsmEngine {
    memtable: MemTable,
    wal: File,
}

impl LsmEngine {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let wal_path = path.as_ref().join(WAL_FILE_NAME);
        let mut memtable = MemTable::new();

        let wal_len = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&wal_path)?
            .metadata()?
            .len();

        if wal_len == 0 {
            let mut wal = OpenOptions::new().append(true).open(&wal_path)?;
            wal::write_header(&mut wal)?;
            wal.flush()?;
            wal.sync_data()?;
        } else {
            let replay_end = replay_wal(&wal_path, &mut memtable)?;
            let wal = OpenOptions::new().write(true).open(&wal_path)?;
            wal.set_len(replay_end)?;
        }

        let wal = OpenOptions::new().append(true).open(&wal_path)?;

        Ok(Self { memtable, wal })
    }

    fn append_record(&mut self, record: &WalRecord) -> Result<()> {
        wal::write_record(&mut self.wal, record)?;
        self.wal.flush()?;
        self.wal.sync_data()?;
        Ok(())
    }
}

impl KeyValueStore for LsmEngine {
    fn put(&mut self, key: Key, value: Value) -> Result<()> {
        let record = WalRecord::Put {
            key: key.clone(),
            value: value.clone(),
        };
        self.append_record(&record)?;
        self.memtable.put(key, value);
        Ok(())
    }

    fn get(&self, key: &[u8]) -> Result<Option<Value>> {
        Ok(self.memtable.get(key).cloned())
    }

    fn delete(&mut self, key: &[u8]) -> Result<()> {
        let record = WalRecord::Delete { key: key.to_vec() };
        self.append_record(&record)?;
        self.memtable.delete(key);
        Ok(())
    }

    fn scan(&self, range: KeyRange) -> Result<Vec<KeyValue>> {
        Ok(self.memtable.scan(range))
    }
}

fn replay_wal(path: &Path, memtable: &mut MemTable) -> Result<u64> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    wal::read_header(&mut reader)?;

    loop {
        let record_start = reader.stream_position()?;

        match wal::read_record(&mut reader)? {
            ReadRecord::Record(record) => apply_record(memtable, record),
            ReadRecord::CleanEof => return Ok(record_start),
            ReadRecord::PartialTail => return Ok(record_start),
        }
    }
}

fn apply_record(memtable: &mut MemTable, record: WalRecord) {
    match record {
        WalRecord::Put { key, value } => memtable.put(key, value),
        WalRecord::Delete { key } => memtable.delete(&key),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write as _;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn put_then_get_returns_value() {
        let (mut engine, path) = open_engine("put-then-get");

        engine.put(b"hello".to_vec(), b"world".to_vec()).unwrap();

        assert_eq!(engine.get(b"hello").unwrap(), Some(b"world".to_vec()));

        drop(engine);
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn put_existing_key_replaces_value() {
        let (mut engine, path) = open_engine("replace");

        engine.put(b"hello".to_vec(), b"old".to_vec()).unwrap();
        engine.put(b"hello".to_vec(), b"new".to_vec()).unwrap();

        assert_eq!(engine.get(b"hello").unwrap(), Some(b"new".to_vec()));

        drop(engine);
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn delete_removes_value_from_active_memtable() {
        let (mut engine, path) = open_engine("delete");

        engine.put(b"hello".to_vec(), b"world".to_vec()).unwrap();
        engine.delete(b"hello").unwrap();

        assert_eq!(engine.get(b"hello").unwrap(), None);

        drop(engine);
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn scan_returns_sorted_memtable_entries() {
        let (mut engine, path) = open_engine("scan");

        engine.put(b"delta".to_vec(), b"4".to_vec()).unwrap();
        engine.put(b"alpha".to_vec(), b"1".to_vec()).unwrap();
        engine.put(b"charlie".to_vec(), b"3".to_vec()).unwrap();
        engine.put(b"bravo".to_vec(), b"2".to_vec()).unwrap();

        let entries = engine
            .scan(KeyRange::between(b"bravo".to_vec(), b"delta".to_vec()))
            .unwrap();

        assert_eq!(
            entries,
            vec![
                KeyValue {
                    key: b"bravo".to_vec(),
                    value: b"2".to_vec(),
                },
                KeyValue {
                    key: b"charlie".to_vec(),
                    value: b"3".to_vec(),
                },
            ]
        );

        drop(engine);
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn reopen_replays_wal_records() {
        let path = unique_temp_path("replay");
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();

        {
            let mut engine = LsmEngine::open(&path).unwrap();
            engine.put(b"alpha".to_vec(), b"1".to_vec()).unwrap();
            engine.put(b"bravo".to_vec(), b"2".to_vec()).unwrap();
        }

        let engine = LsmEngine::open(&path).unwrap();

        assert_eq!(engine.get(b"alpha").unwrap(), Some(b"1".to_vec()));
        assert_eq!(engine.get(b"bravo").unwrap(), Some(b"2".to_vec()));

        drop(engine);
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn reopen_replays_delete_as_tombstone() {
        let path = unique_temp_path("replay-delete");
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();

        {
            let mut engine = LsmEngine::open(&path).unwrap();
            engine.put(b"alpha".to_vec(), b"1".to_vec()).unwrap();
            engine.delete(b"alpha").unwrap();
        }

        let engine = LsmEngine::open(&path).unwrap();

        assert_eq!(engine.get(b"alpha").unwrap(), None);

        drop(engine);
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn reopen_ignores_partial_tail_and_allows_new_appends() {
        let path = unique_temp_path("partial-tail");
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        let wal_path = path.join(WAL_FILE_NAME);

        {
            let mut file = File::create(&wal_path).unwrap();
            wal::write_header(&mut file).unwrap();
            wal::write_record(
                &mut file,
                &WalRecord::Put {
                    key: b"alpha".to_vec(),
                    value: b"1".to_vec(),
                },
            )
            .unwrap();

            let mut partial = wal::encode_record(&WalRecord::Put {
                key: b"bravo".to_vec(),
                value: b"2".to_vec(),
            })
            .unwrap();
            partial.pop();
            file.write_all(&partial).unwrap();
        }

        {
            let mut engine = LsmEngine::open(&path).unwrap();

            assert_eq!(engine.get(b"alpha").unwrap(), Some(b"1".to_vec()));
            assert_eq!(engine.get(b"bravo").unwrap(), None);

            engine.put(b"charlie".to_vec(), b"3".to_vec()).unwrap();
        }

        let engine = LsmEngine::open(&path).unwrap();

        assert_eq!(engine.get(b"alpha").unwrap(), Some(b"1".to_vec()));
        assert_eq!(engine.get(b"bravo").unwrap(), None);
        assert_eq!(engine.get(b"charlie").unwrap(), Some(b"3".to_vec()));

        drop(engine);
        fs::remove_dir_all(path).unwrap();
    }

    fn open_engine(name: &str) -> (LsmEngine, PathBuf) {
        let path = unique_temp_path(name);
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();

        (LsmEngine::open(&path).unwrap(), path)
    }

    fn unique_temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("bloomy-lsm-{name}-{}", std::process::id()))
    }
}
