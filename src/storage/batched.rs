use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::{StorageData, StorageEntry};

/// Batched storage implementation that reduces I/O operations
/// by accumulating writes and flushing them periodically or when a threshold is reached
#[derive(Clone)]
pub struct BatchedStorage {
    root_path: PathBuf,
    storage_path: PathBuf,
    inner: Arc<Mutex<BatchedStorageInner>>,
}

struct BatchedStorageInner {
    pending_writes: HashMap<String, StorageEntry>,
    pending_deletes: Vec<String>,
    last_flush: Instant,
    cached_data: Option<StorageData>,
    cache_valid_until: Instant,
}

impl BatchedStorage {
    const BATCH_SIZE: usize = 50;
    const FLUSH_INTERVAL: Duration = Duration::from_secs(1);
    const CACHE_DURATION: Duration = Duration::from_secs(5);

    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        let root_path = project_root.as_ref().to_path_buf();
        let storage_path = root_path.join(".whiteout").join("local.toml");
        
        Ok(Self {
            root_path,
            storage_path,
            inner: Arc::new(Mutex::new(BatchedStorageInner {
                pending_writes: HashMap::new(),
                pending_deletes: Vec::new(),
                last_flush: Instant::now(),
                cached_data: None,
                cache_valid_until: Instant::now(),
            })),
        })
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
        
        let mut inner = self.inner.lock().unwrap();
        inner.pending_writes.insert(storage_key, entry);
        
        // Auto-flush if threshold reached
        if inner.pending_writes.len() >= Self::BATCH_SIZE ||
           inner.last_flush.elapsed() >= Self::FLUSH_INTERVAL {
            drop(inner); // Release lock before flush
            self.flush()?;
        }
        
        Ok(())
    }

    pub fn get_value(&self, file_path: &Path, key: &str) -> Result<String> {
        let storage_key = self.make_storage_key(file_path, key);
        
        let inner = self.inner.lock().unwrap();
        
        // Check pending writes first
        if let Some(entry) = inner.pending_writes.get(&storage_key) {
            return Ok(entry.value.clone());
        }
        
        // Check if key is pending deletion
        if inner.pending_deletes.contains(&storage_key) {
            return Err(anyhow::anyhow!("Value not found for key: {}", storage_key));
        }
        
        drop(inner); // Release lock before loading
        
        // Load from disk with caching
        let data = self.load_data_cached()?;
        data.entries
            .get(&storage_key)
            .map(|e| e.value.clone())
            .ok_or_else(|| anyhow::anyhow!("Value not found for key: {}", storage_key))
    }

    pub fn remove_value(&self, file_path: &Path, key: &str) -> Result<()> {
        let storage_key = self.make_storage_key(file_path, key);
        
        let mut inner = self.inner.lock().unwrap();
        inner.pending_writes.remove(&storage_key);
        inner.pending_deletes.push(storage_key);
        
        // Auto-flush if threshold reached
        if inner.pending_deletes.len() >= Self::BATCH_SIZE {
            drop(inner);
            self.flush()?;
        }
        
        Ok(())
    }

    pub fn flush(&self) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        
        if inner.pending_writes.is_empty() && inner.pending_deletes.is_empty() {
            return Ok(());
        }
        
        // Load current data
        let mut data = if self.storage_path.exists() {
            let content = fs::read_to_string(&self.storage_path)
                .context("Failed to read storage file")?;
            toml::from_str(&content).context("Failed to parse storage file")?
        } else {
            StorageData {
                version: "0.1.0".to_string(),
                entries: HashMap::new(),
            }
        };
        
        // Apply pending writes
        for (key, entry) in inner.pending_writes.drain() {
            data.entries.insert(key, entry);
        }
        
        // Apply pending deletes
        for key in inner.pending_deletes.drain(..) {
            data.entries.remove(&key);
        }
        
        // Write to disk
        let content = toml::to_string_pretty(&data)
            .context("Failed to serialize storage")?;
        
        fs::create_dir_all(self.storage_path.parent().unwrap())
            .context("Failed to create storage directory")?;
        
        // Atomic write with temp file
        let temp_path = self.storage_path.with_extension("tmp");
        fs::write(&temp_path, content)
            .context("Failed to write temp storage file")?;
        fs::rename(&temp_path, &self.storage_path)
            .context("Failed to rename storage file")?;
        
        // Update cache
        inner.cached_data = Some(data);
        inner.cache_valid_until = Instant::now() + Self::CACHE_DURATION;
        inner.last_flush = Instant::now();
        
        Ok(())
    }

    fn load_data_cached(&self) -> Result<StorageData> {
        let mut inner = self.inner.lock().unwrap();
        
        // Return cached data if still valid
        if let Some(ref data) = inner.cached_data {
            if Instant::now() < inner.cache_valid_until {
                return Ok(data.clone());
            }
        }
        
        // Load from disk
        let data = if self.storage_path.exists() {
            let content = fs::read_to_string(&self.storage_path)
                .context("Failed to read storage file")?;
            toml::from_str(&content).context("Failed to parse storage file")?
        } else {
            StorageData {
                version: "0.1.0".to_string(),
                entries: HashMap::new(),
            }
        };
        
        // Update cache
        inner.cached_data = Some(data.clone());
        inner.cache_valid_until = Instant::now() + Self::CACHE_DURATION;
        
        Ok(data)
    }

    fn make_storage_key(&self, file_path: &Path, key: &str) -> String {
        let relative_path = file_path
            .strip_prefix(&self.root_path)
            .unwrap_or(file_path);
        
        format!("{}::{}", relative_path.display(), key)
    }
}

impl Drop for BatchedStorage {
    fn drop(&mut self) {
        // Best effort flush on drop
        let _ = self.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_batched_writes() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = BatchedStorage::new(temp_dir.path())?;
        
        // Write multiple values without triggering flush
        for i in 0..10 {
            let file_name = format!("test{}.rs", i);
            let file_path = Path::new(&file_name);
            let value = format!("value{}", i);
            storage.store_value(file_path, "key", &value)?;
        }
        
        // Values should be readable from pending writes
        let file_path = Path::new("test5.rs");
        assert_eq!(storage.get_value(file_path, "key")?, "value5");
        
        // Force flush
        storage.flush()?;
        
        // Values should still be readable after flush
        assert_eq!(storage.get_value(file_path, "key")?, "value5");
        
        Ok(())
    }

    #[test]
    fn test_auto_flush() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = BatchedStorage::new(temp_dir.path())?;
        
        // Write enough values to trigger auto-flush
        for i in 0..BatchedStorage::BATCH_SIZE + 1 {
            let file_name = format!("test{}.rs", i);
            let file_path = Path::new(&file_name);
            let value = format!("value{}", i);
            storage.store_value(file_path, "key", &value)?;
        }
        
        // Check that file was written
        assert!(temp_dir.path().join(".whiteout/local.toml").exists());
        
        Ok(())
    }

    #[test]
    fn test_cached_reads() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = BatchedStorage::new(temp_dir.path())?;
        
        let file_path = Path::new("test.rs");
        storage.store_value(file_path, "key", "value")?;
        storage.flush()?;
        
        // Multiple reads should use cache
        for _ in 0..10 {
            assert_eq!(storage.get_value(file_path, "key")?, "value");
        }
        
        Ok(())
    }
}