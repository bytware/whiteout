// Optimized local storage with caching and batch operations
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;

use super::{StorageData, StorageEntry};

// In-memory cache for frequently accessed values
type StorageCache = Arc<RwLock<HashMap<String, String>>>;

static STORAGE_CACHE: Lazy<StorageCache> = Lazy::new(|| {
    Arc::new(RwLock::new(HashMap::new()))
});

#[derive(Debug, Clone)]
pub struct LocalStorage {
    root_path: PathBuf,
    storage_path: PathBuf,
    cache: StorageCache,
    // Buffer for batch writes
    write_buffer: Arc<RwLock<Vec<(String, StorageEntry)>>>,
}

impl LocalStorage {
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        let root_path = project_root.as_ref().to_path_buf();
        let storage_path = root_path.join(".whiteout").join("local.toml");
        
        let storage = Self {
            root_path,
            storage_path,
            cache: STORAGE_CACHE.clone(),
            write_buffer: Arc::new(RwLock::new(Vec::new())),
        };
        
        // Pre-load cache if storage exists
        if storage.storage_path.exists() {
            storage.preload_cache()?;
        }
        
        Ok(storage)
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
    
    // Preload frequently accessed values into cache
    fn preload_cache(&self) -> Result<()> {
        let data = self.load_data()?;
        let mut cache = self.cache.write().unwrap();
        
        for (key, entry) in data.entries.iter() {
            cache.insert(key.clone(), entry.value.clone());
        }
        
        Ok(())
    }

    // Batch store operation for multiple values
    pub fn store_values_batch(&self, values: Vec<(PathBuf, String, String)>) -> Result<()> {
        let mut data = self.load_data()?;
        let mut cache = self.cache.write().unwrap();
        
        for (file_path, key, value) in values {
            let storage_key = self.make_storage_key(&file_path, &key);
            
            let entry = StorageEntry {
                file_path: file_path.clone(),
                key: key.clone(),
                value: value.clone(),
                encrypted: false,
                timestamp: chrono::Utc::now(),
            };
            
            data.entries.insert(storage_key.clone(), entry);
            cache.insert(storage_key, value);
        }
        
        // Single write operation for all values
        self.write_data(&data)?;
        Ok(())
    }

    pub fn store_value(
        &self,
        file_path: &Path,
        key: &str,
        value: &str,
    ) -> Result<()> {
        let storage_key = self.make_storage_key(file_path, key);
        
        // Update cache immediately
        {
            let mut cache = self.cache.write().unwrap();
            cache.insert(storage_key.clone(), value.to_string());
        }
        
        // Add to write buffer for batch writing
        {
            let mut buffer = self.write_buffer.write().unwrap();
            buffer.push((
                storage_key.clone(),
                StorageEntry {
                    file_path: file_path.to_path_buf(),
                    key: key.to_string(),
                    value: value.to_string(),
                    encrypted: false,
                    timestamp: chrono::Utc::now(),
                }
            ));
            
            // Flush buffer if it gets too large
            if buffer.len() >= 100 {
                drop(buffer); // Release lock before flushing
                self.flush_write_buffer()?;
            }
        }
        
        Ok(())
    }
    
    // Flush write buffer to disk
    pub fn flush_write_buffer(&self) -> Result<()> {
        let mut buffer = self.write_buffer.write().unwrap();
        if buffer.is_empty() {
            return Ok(());
        }
        
        let mut data = self.load_data()?;
        
        for (key, entry) in buffer.drain(..) {
            data.entries.insert(key, entry);
        }
        
        self.write_data(&data)?;
        Ok(())
    }

    pub fn get_value(&self, file_path: &Path, key: &str) -> Result<String> {
        let storage_key = self.make_storage_key(file_path, key);
        
        // Check cache first
        {
            let cache = self.cache.read().unwrap();
            if let Some(value) = cache.get(&storage_key) {
                return Ok(value.clone());
            }
        }
        
        // Fall back to disk
        let data = self.load_data()?;
        let value = data.entries
            .get(&storage_key)
            .map(|e| e.value.clone())
            .ok_or_else(|| anyhow::anyhow!("Value not found for key: {}", storage_key))?;
        
        // Update cache for next access
        {
            let mut cache = self.cache.write().unwrap();
            cache.insert(storage_key, value.clone());
        }
        
        Ok(value)
    }

    pub fn remove_value(&self, file_path: &Path, key: &str) -> Result<()> {
        let storage_key = self.make_storage_key(file_path, key);
        
        // Remove from cache
        {
            let mut cache = self.cache.write().unwrap();
            cache.remove(&storage_key);
        }
        
        let mut data = self.load_data()?;
        data.entries.remove(&storage_key);
        
        self.write_data(&data)?;
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
    
    fn write_data(&self, data: &StorageData) -> Result<()> {
        let content = toml::to_string_pretty(data)
            .context("Failed to serialize storage")?;
        
        fs::create_dir_all(self.storage_path.parent().unwrap())
            .context("Failed to create storage directory")?;
        
        // Atomic write using temp file and rename
        let temp_path = self.storage_path.with_extension("tmp");
        fs::write(&temp_path, content)
            .context("Failed to write temp storage file")?;
        fs::rename(&temp_path, &self.storage_path)
            .context("Failed to rename temp storage file")?;
        
        Ok(())
    }

    fn make_storage_key(&self, file_path: &Path, key: &str) -> String {
        let relative_path = file_path
            .strip_prefix(&self.root_path)
            .unwrap_or(file_path);
        
        format!("{}::{}", relative_path.display(), key)
    }
}

// Ensure write buffer is flushed on drop
impl Drop for LocalStorage {
    fn drop(&mut self) {
        let _ = self.flush_write_buffer();
    }
}