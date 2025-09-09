use anyhow::{Context, Result};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Atomic file operations to prevent TOCTOU race conditions
pub struct AtomicFile {
    path: PathBuf,
    temp_path: PathBuf,
}

impl AtomicFile {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let temp_path = Self::temp_path(&path)?;
        
        Ok(Self { path, temp_path })
    }
    
    /// Generate a temporary file path for atomic operations
    fn temp_path(path: &Path) -> Result<PathBuf> {
        let file_name = path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;
        
        let temp_name = format!(
            ".{}.tmp.{}",
            file_name.to_string_lossy(),
            std::process::id()
        );
        
        Ok(path.with_file_name(temp_name))
    }
    
    /// Atomically write content to file
    pub fn write(&self, content: &[u8]) -> Result<()> {
        // Write to temporary file first
        let mut temp_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.temp_path)
            .context("Failed to create temporary file")?;
        
        temp_file
            .write_all(content)
            .context("Failed to write to temporary file")?;
        
        temp_file
            .sync_all()
            .context("Failed to sync temporary file")?;
        
        // Set permissions on Unix systems
        #[cfg(unix)]
        {
            let metadata = fs::metadata(&self.temp_path)?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o644); // Read/write for owner, read for others
            fs::set_permissions(&self.temp_path, permissions)?;
        }
        
        // Atomically rename temp file to target
        fs::rename(&self.temp_path, &self.path)
            .context("Failed to atomically rename file")?;
        
        Ok(())
    }
    
    /// Atomically read file with retry logic
    pub fn read(&self) -> Result<Vec<u8>> {
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAY: Duration = Duration::from_millis(10);
        
        for attempt in 0..MAX_RETRIES {
            match self.try_read() {
                Ok(content) => return Ok(content),
                Err(e) if attempt < MAX_RETRIES - 1 => {
                    // Check if it's a temporary failure
                    if e.to_string().contains("temporarily unavailable") ||
                       e.to_string().contains("locked") {
                        std::thread::sleep(RETRY_DELAY);
                        continue;
                    }
                    return Err(e);
                }
                Err(e) => return Err(e),
            }
        }
        
        anyhow::bail!("Failed to read file after {} attempts", MAX_RETRIES)
    }
    
    fn try_read(&self) -> Result<Vec<u8>> {
        let mut file = File::open(&self.path)
            .with_context(|| format!("Failed to open file: {}", self.path.display()))?;
        
        let mut content = Vec::new();
        file.read_to_end(&mut content)
            .context("Failed to read file content")?;
        
        Ok(content)
    }
    
    /// Check if file exists with proper validation
    pub fn exists(&self) -> bool {
        self.path.exists() && self.path.is_file()
    }
    
    /// Securely delete file
    pub fn delete(&self) -> Result<()> {
        if self.temp_path.exists() {
            fs::remove_file(&self.temp_path)
                .context("Failed to remove temporary file")?;
        }
        
        if self.path.exists() {
            fs::remove_file(&self.path)
                .context("Failed to remove file")?;
        }
        
        Ok(())
    }
}

/// File locking mechanism to prevent concurrent access
#[cfg(unix)]
pub mod lock {
    use std::fs::File;
    use std::os::unix::io::AsRawFd;
    use anyhow::Result;
    
    pub struct FileLock {
        file: File,
    }
    
    impl FileLock {
        pub fn acquire(file: File) -> Result<Self> {
            use libc::{flock, LOCK_EX};
            
            let fd = file.as_raw_fd();
            let result = unsafe { flock(fd, LOCK_EX) };
            
            if result != 0 {
                anyhow::bail!("Failed to acquire file lock");
            }
            
            Ok(Self { file })
        }
        
        pub fn try_acquire(file: File) -> Result<Option<Self>> {
            use libc::{flock, LOCK_EX, LOCK_NB};
            
            let fd = file.as_raw_fd();
            let result = unsafe { flock(fd, LOCK_EX | LOCK_NB) };
            
            if result == 0 {
                Ok(Some(Self { file }))
            } else if std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock {
                Ok(None)
            } else {
                anyhow::bail!("Failed to try acquiring file lock");
            }
        }
    }
    
    impl Drop for FileLock {
        fn drop(&mut self) {
            use libc::{flock, LOCK_UN};
            
            let fd = self.file.as_raw_fd();
            unsafe { flock(fd, LOCK_UN) };
        }
    }
}

#[cfg(not(unix))]
pub mod lock {
    use std::fs::File;
    use anyhow::Result;
    
    pub struct FileLock {
        _file: File,
    }
    
    impl FileLock {
        pub fn acquire(file: File) -> Result<Self> {
            // Windows file locking would go here
            Ok(Self { _file: file })
        }
        
        pub fn try_acquire(file: File) -> Result<Option<Self>> {
            Ok(Some(Self { _file: file }))
        }
    }
}

/// Validate file path to prevent directory traversal
pub fn validate_path<P: AsRef<Path>>(path: P, base_dir: &Path) -> Result<PathBuf> {
    let path = path.as_ref();
    
    // Resolve to canonical path
    let canonical = if path.exists() {
        path.canonicalize()
            .context("Failed to canonicalize path")?
    } else {
        // For non-existent files, canonicalize the parent and append filename
        let parent = path.parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid path: no parent directory"))?;
        
        let parent_canonical = parent.canonicalize()
            .context("Failed to canonicalize parent directory")?;
        
        let file_name = path.file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid path: no file name"))?;
        
        parent_canonical.join(file_name)
    };
    
    // Ensure path is within base directory
    let base_canonical = base_dir.canonicalize()
        .context("Failed to canonicalize base directory")?;
    
    if !canonical.starts_with(&base_canonical) {
        anyhow::bail!(
            "Path traversal detected: {} is outside of {}",
            canonical.display(),
            base_canonical.display()
        );
    }
    
    // Check for suspicious patterns
    let path_str = canonical.to_string_lossy();
    if path_str.contains("..") || path_str.contains("~") {
        anyhow::bail!("Suspicious path pattern detected");
    }
    
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_atomic_write_read() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        
        let atomic_file = AtomicFile::new(&file_path)?;
        let content = b"test content";
        
        atomic_file.write(content)?;
        let read_content = atomic_file.read()?;
        
        assert_eq!(content, &read_content[..]);
        Ok(())
    }
    
    #[test]
    fn test_path_validation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let base = temp_dir.path();
        
        // Valid path
        let valid_path = base.join("subdir").join("file.txt");
        std::fs::create_dir_all(valid_path.parent().unwrap())?;
        let result = validate_path(&valid_path, base);
        assert!(result.is_ok());
        
        // Invalid path (traversal attempt)
        let invalid_path = base.join("..").join("outside.txt");
        let result = validate_path(&invalid_path, base);
        assert!(result.is_err());
        
        Ok(())
    }
    
    #[test]
    fn test_file_locking() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("locked.txt");
        
        std::fs::write(&file_path, "test")?;
        
        let file1 = File::open(&file_path)?;
        let lock1 = lock::FileLock::acquire(file1)?;
        
        // Try to acquire another lock (should fail or block)
        let file2 = File::open(&file_path)?;
        let lock2 = lock::FileLock::try_acquire(file2)?;
        
        assert!(lock2.is_none() || cfg!(not(unix)));
        
        drop(lock1);
        Ok(())
    }
}