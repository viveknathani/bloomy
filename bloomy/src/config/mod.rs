use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::error::Error;
use crate::error::Result;

pub const DEFAULT_MEMTABLE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BloomyConfig {
    pub storage_path: PathBuf,
    pub memtable_bytes: usize,
}

impl Default for BloomyConfig {
    fn default() -> Self {
        Self {
            storage_path: PathBuf::from("./bloomy-data"),
            memtable_bytes: DEFAULT_MEMTABLE_BYTES,
        }
    }
}

impl BloomyConfig {
    pub fn load_or_create(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            let config = Self::default();
            config.write_to(path)?;
            return Ok(config);
        }

        let contents = fs::read_to_string(path)?;
        let config = serde_json::from_str(&contents)?;
        validate(&config)?;
        Ok(config)
    }

    pub fn write_to(&self, path: impl AsRef<Path>) -> Result<()> {
        validate(self)?;

        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, format!("{contents}\n"))?;
        Ok(())
    }
}

pub fn default_config_path() -> Result<PathBuf> {
    let home = env::var_os("HOME").ok_or_else(|| Error::Message("HOME is not set".to_string()))?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("bloomy")
        .join("bloomy.json"))
}

fn validate(config: &BloomyConfig) -> Result<()> {
    if config.storage_path.as_os_str().is_empty() {
        return Err(Error::Message("storage_path must not be empty".to_string()));
    }

    if config.memtable_bytes == 0 {
        return Err(Error::Message(
            "memtable_bytes must be greater than zero".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn load_or_create_writes_default_config_when_missing() {
        let path = unique_temp_path("missing");

        let config = BloomyConfig::load_or_create(&path).unwrap();

        assert_eq!(config.memtable_bytes, DEFAULT_MEMTABLE_BYTES);
        assert!(path.exists());

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn load_or_create_reads_existing_config() {
        let path = unique_temp_path("existing");
        fs::write(
            &path,
            r#"{
  "storage_path": "./tmp-data",
  "memtable_bytes": 1024
}
"#,
        )
        .unwrap();

        let config = BloomyConfig::load_or_create(&path).unwrap();

        assert_eq!(config.storage_path, PathBuf::from("./tmp-data"));
        assert_eq!(config.memtable_bytes, 1024);

        fs::remove_file(path).unwrap();
    }

    fn unique_temp_path(name: &str) -> PathBuf {
        env::temp_dir().join(format!("bloomy-{name}-{}.json", std::process::id()))
    }
}
