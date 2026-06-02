use std::fs;
use std::path::PathBuf;

use crate::engine::lsm::LsmEngine;
use crate::error::Result;
pub use crate::types::{Key, KeyRange, KeyValue, Value};
use crate::{config::BloomyConfig, config::DEFAULT_MEMTABLE_BYTES};

#[derive(Debug, Clone)]
pub struct BloomyOptions {
    pub path: PathBuf,
    pub memtable_bytes: usize,
}

impl BloomyOptions {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            memtable_bytes: DEFAULT_MEMTABLE_BYTES,
        }
    }
}

impl From<BloomyConfig> for BloomyOptions {
    fn from(config: BloomyConfig) -> Self {
        Self {
            path: config.storage_path,
            memtable_bytes: config.memtable_bytes,
        }
    }
}

#[derive(Debug)]
pub struct Bloomy {
    engine: LsmEngine,
}

impl Bloomy {
    pub fn open(options: BloomyOptions) -> Result<Self> {
        fs::create_dir_all(&options.path)?;

        Ok(Self {
            engine: LsmEngine::default(),
        })
    }

    pub fn put(&mut self, key: impl Into<Key>, value: impl Into<Value>) -> Result<()> {
        self.engine.put(key.into(), value.into())
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Value>> {
        self.engine.get(key)
    }

    pub fn delete(&mut self, key: &[u8]) -> Result<()> {
        self.engine.delete(key)
    }

    pub fn scan(&self, range: KeyRange) -> Result<Vec<KeyValue>> {
        self.engine.scan(range)
    }

    pub fn close(self) -> Result<()> {
        Ok(())
    }
}

pub trait KeyValueStore {
    fn put(&mut self, key: Key, value: Value) -> Result<()>;

    fn get(&self, key: &[u8]) -> Result<Option<Value>>;

    fn delete(&mut self, key: &[u8]) -> Result<()>;

    fn scan(&self, range: KeyRange) -> Result<Vec<KeyValue>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_creates_storage_directory() {
        let path = unique_temp_path("storage-dir");
        let _ = fs::remove_dir_all(&path);

        let _bloomy = Bloomy::open(BloomyOptions::new(&path)).unwrap();

        assert!(path.is_dir());

        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn close_succeeds_for_open_store() {
        let path = unique_temp_path("close");
        let _ = fs::remove_dir_all(&path);

        let bloomy = Bloomy::open(BloomyOptions::new(&path)).unwrap();

        bloomy.close().unwrap();
        fs::remove_dir_all(path).unwrap();
    }

    fn unique_temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("bloomy-{name}-{}", std::process::id()))
    }
}
