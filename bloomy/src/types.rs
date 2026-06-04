pub type Key = Vec<u8>;

pub type Value = Vec<u8>;

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct KeyRange {
    pub start: Option<Key>,
    pub end: Option<Key>,
}

impl KeyRange {
    pub fn all() -> Self {
        Self::default()
    }

    pub fn from(start: impl Into<Key>) -> Self {
        Self {
            start: Some(start.into()),
            end: None,
        }
    }

    pub fn between(start: impl Into<Key>, end: impl Into<Key>) -> Self {
        Self {
            start: Some(start.into()),
            end: Some(end.into()),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct KeyValue {
    pub key: Key,
    pub value: Value,
}
