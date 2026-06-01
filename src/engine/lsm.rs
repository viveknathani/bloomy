use crate::api::{Key, KeyRange, KeyValue, KeyValueStore, Value};
use crate::error::{Error, Result};

#[derive(Debug, Default)]
pub struct LsmEngine;

impl KeyValueStore for LsmEngine {
    fn put(&mut self, _key: Key, _value: Value) -> Result<()> {
        Err(Error::Unsupported("lsm put"))
    }

    fn get(&self, _key: &[u8]) -> Result<Option<Value>> {
        Err(Error::Unsupported("lsm get"))
    }

    fn delete(&mut self, _key: &[u8]) -> Result<()> {
        Err(Error::Unsupported("lsm delete"))
    }

    fn scan(&self, _range: KeyRange) -> Result<Vec<KeyValue>> {
        Err(Error::Unsupported("lsm scan"))
    }
}
