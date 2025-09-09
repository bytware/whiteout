use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::{StorageData, StorageEntry};

#[derive(Debug, Clone)]
pub struct LocalStorage {
    root_path: PathBuf,
    storage_path: PathBuf,
}

impl LocalStorage {
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        let root_path = project_root.as_ref().to_path_buf();
        let storage_path = root_path.join(".whiteout").join("local.toml");
        
        Ok(Self {
            root_path,
            storage_path,
        })
    }

    pub fn init(project_root: impl AsRef<Path>) -> Result<()> {
        let whiteout_dir = project_root.as_ref().join(".whiteout");
        fs::create_dir_all(&whiteout_dir).context("Failed to create .whiteout directory")?;
        
        let gitignore_path = whiteout_dir.join(".gitignore");
        if !gitignore_path.exists() {
            fs::write(&gitignore_path, "local.toml\n*.bak\n")
                .context("Failed to create .gitignore")?;
        }
        
        let storage_path = whiteout_dir.join("local.toml");
        if !storage_path.exists() {
            let initial_data = StorageData {
                version: "0.1.0".to_string(),
                entries: HashMap::new(),
            };
            let content = toml::to_string_pretty(&initial_data)
                .context("Failed to serialize initial storage")?;
            fs::write(&storage_path, content).context("Failed to write initial storage")?;
        }
        
        Ok(())
    }

    pub fn store_value(
        &self,
        file_path: &Path,
        key: &str,
        value: &str,
    ) -> Result<()> {
        let storage_key = self.make_storage_key(file_path, key);
        
        let entry = StorageEntry {
            file_path: file_path.to_path_buf(),
            key: key.to_string(),
            value: value.to_string(),
            encrypted: false,
            timestamp: chrono::Utc::now(),
        };
        
        let mut data = self.load_data()?;
        data.entries.insert(storage_key, entry);
        
        let content = toml::to_string_pretty(&data)
            .context("Failed to serialize storage")?;
        
        fs::create_dir_all(self.storage_path.parent().unwrap())
            .context("Failed to create storage directory")?;
        
        fs::write(&self.storage_path, content)
            .context("Failed to write storage file")?;
        
        Ok(())
    }

    pub fn get_value(&self, file_path: &Path, key: &str) -> Result<String> {
        let storage_key = self.make_storage_key(file_path, key);
        let data = self.load_data()?;
        
        data.entries
            .get(&storage_key)
            .map(|e| e.value.clone())
            .ok_or_else(|| anyhow::anyhow!("Value not found for key: {}", storage_key))
    }

    pub fn remove_value(&self, file_path: &Path, key: &str) -> Result<()> {
        let storage_key = self.make_storage_key(file_path, key);
        
        let mut data = self.load_data()?;
        data.entries.remove(&storage_key);
        
        let content = toml::to_string_pretty(&data)
            .context("Failed to serialize storage")?;
        
        fs::write(&self.storage_path, content)
            .context("Failed to write storage file")?;
        
        Ok(())
    }

    pub fn list_values(&self, file_path: Option<&Path>) -> Result<Vec<StorageEntry>> {
        let data = self.load_data()?;
        Ok(data
            .entries
            .values()
            .filter(|e| {
                file_path.map_or(true, |fp| e.file_path == fp)
            })
            .cloned()
            .collect())
    }
    
    fn load_data(&self) -> Result<StorageData> {
        if self.storage_path.exists() {
            let content = fs::read_to_string(&self.storage_path)
                .context("Failed to read storage file")?;
            toml::from_str(&content).context("Failed to parse storage file")
        } else {
            Ok(StorageData {
                version: "0.1.0".to_string(),
                entries: HashMap::new(),
            })
        }
    }

    fn make_storage_key(&self, file_path: &Path, key: &str) -> String {
        let relative_path = file_path
            .strip_prefix(&self.root_path)
            .unwrap_or(file_path);
        
        format!("{}::{}", relative_path.display(), key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_storage_init() -> Result<()> {
        let temp_dir = TempDir::new()?;
        LocalStorage::init(temp_dir.path())?;
        
        assert!(temp_dir.path().join(".whiteout").exists());
        assert!(temp_dir.path().join(".whiteout/.gitignore").exists());
        assert!(temp_dir.path().join(".whiteout/local.toml").exists());
        
        Ok(())
    }

    #[test]
    fn test_store_and_get_value() -> Result<()> {
        let temp_dir = TempDir::new()?;
        LocalStorage::init(temp_dir.path())?;
        let storage = LocalStorage::new(temp_dir.path())?;
        
        let file_path = Path::new("test.rs");
        storage.store_value(file_path, "test_key", "test_value")?;
        
        let value = storage.get_value(file_path, "test_key")?;
        assert_eq!(value, "test_value");
        
        Ok(())
    }

    #[test]
    fn test_remove_value() -> Result<()> {
        let temp_dir = TempDir::new()?;
        LocalStorage::init(temp_dir.path())?;
        let storage = LocalStorage::new(temp_dir.path())?;
        
        let file_path = Path::new("test.rs");
        storage.store_value(file_path, "test_key", "test_value")?;
        storage.remove_value(file_path, "test_key")?;
        
        assert!(storage.get_value(file_path, "test_key").is_err());
        
        Ok(())
    }
}