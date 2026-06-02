use crate::api::KeyValueStore;
use crate::error::Result;
use crate::storage::memtable::MemTable;
use crate::types::Key;
use crate::types::KeyRange;
use crate::types::KeyValue;
use crate::types::Value;

#[derive(Debug)]
pub struct LsmEngine {
    memtable: MemTable,
}

impl Default for LsmEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LsmEngine {
    pub fn new() -> Self {
        Self {
            memtable: MemTable::new(),
        }
    }
}

impl KeyValueStore for LsmEngine {
    fn put(&mut self, key: Key, value: Value) -> Result<()> {
        self.memtable.put(key, value);
        Ok(())
    }

    fn get(&self, key: &[u8]) -> Result<Option<Value>> {
        Ok(self.memtable.get(key).cloned())
    }

    fn delete(&mut self, key: &[u8]) -> Result<()> {
        self.memtable.delete(key);
        Ok(())
    }

    fn scan(&self, range: KeyRange) -> Result<Vec<KeyValue>> {
        Ok(self.memtable.scan(range))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_then_get_returns_value() {
        let mut engine = LsmEngine::new();

        engine.put(b"hello".to_vec(), b"world".to_vec()).unwrap();

        assert_eq!(engine.get(b"hello").unwrap(), Some(b"world".to_vec()));
    }

    #[test]
    fn put_existing_key_replaces_value() {
        let mut engine = LsmEngine::new();

        engine.put(b"hello".to_vec(), b"old".to_vec()).unwrap();
        engine.put(b"hello".to_vec(), b"new".to_vec()).unwrap();

        assert_eq!(engine.get(b"hello").unwrap(), Some(b"new".to_vec()));
    }

    #[test]
    fn delete_removes_value_from_active_memtable() {
        let mut engine = LsmEngine::new();

        engine.put(b"hello".to_vec(), b"world".to_vec()).unwrap();
        engine.delete(b"hello").unwrap();

        assert_eq!(engine.get(b"hello").unwrap(), None);
    }

    #[test]
    fn scan_returns_sorted_memtable_entries() {
        let mut engine = LsmEngine::new();

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
    }
}
