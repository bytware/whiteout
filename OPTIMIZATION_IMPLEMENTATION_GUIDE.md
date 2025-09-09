# Whiteout Optimization Implementation Guide

This document provides concrete implementation examples for the critical optimizations identified in the second-pass audit.

## Priority 1: Memory Layout Optimizations

### 1. Enum Layout Optimization

**Problem**: Current `Decoration` enum wastes memory due to poor layout.

**Implementation**:

```rust
// src/parser/mod.rs - Optimized enum layout
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Decoration {
    Inline {
        line: u32,                    // Reduced from usize (4 bytes vs 8)
        local_value: Arc<str>,        // Shared string data  
        committed_value: Arc<str>,
    },
    Block {
        start_line: u32,
        end_line: u32, 
        local_content: Arc<str>,
        committed_content: Arc<str>,
    },
    Partial {
        line: u32,
        replacements: Box<[PartialReplacement]>, // Fixed-size allocation
    },
}

// Memory layout improvement: 
// Before: 64+ bytes per variant (due to largest Block)
// After: ~32 bytes per variant + shared string data
```

### 2. String Interning System

**Implementation**:

```rust
// src/parser/interning.rs - New file
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct StringInterner {
    strings: Arc<Mutex<HashMap<String, Arc<str>>>>,
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            strings: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub fn intern(&self, s: &str) -> Arc<str> {
        let mut strings = self.strings.lock().unwrap();
        if let Some(interned) = strings.get(s) {
            Arc::clone(interned)
        } else {
            let interned: Arc<str> = Arc::from(s);
            strings.insert(s.to_string(), Arc::clone(&interned));
            interned
        }
    }
}

// Usage in parsers
impl InlineParser {
    pub fn new_with_interner(interner: StringInterner) -> Self {
        Self { interner, .. }
    }
    
    pub fn parse(&self, content: &str) -> Result<Vec<Decoration>, WhiteoutError> {
        let mut decorations = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            if let Some(captures) = INLINE_PATTERN.captures(line) {
                let local_value = self.interner.intern(captures.get(1).unwrap().as_str());
                let committed_value = self.interner.intern(captures.get(2).unwrap().as_str());
                
                decorations.push(Decoration::Inline {
                    line: line_num as u32 + 1,
                    local_value,
                    committed_value,
                });
            }
        }
        
        Ok(decorations)
    }
}
```

## Priority 2: Custom Error Types

### 3. Structured Error Hierarchy

**Implementation**:

```rust
// src/error.rs - New file
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WhiteoutError {
    #[error("Parse error at line {line} in {file}: {message}")]
    Parse {
        line: usize,
        file: PathBuf, 
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    #[error("Storage operation failed")]
    Storage(#[from] StorageError),
    
    #[error("Configuration error")]
    Config(#[from] ConfigError),
    
    #[error("I/O operation failed: {operation}")]
    Io {
        operation: String,
        #[source]
        source: std::io::Error,
    },
    
    #[error("Git integration failed: {message}")]
    Git { message: String },
    
    #[error("Encryption operation failed")]
    Encryption(#[from] EncryptionError),
}

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Key '{key}' not found in storage")]
    KeyNotFound { key: String },
    
    #[error("Storage file is corrupted: {reason}")]
    Corrupted { reason: String },
    
    #[error("Concurrent access conflict")]
    Conflict,
    
    #[error("Storage file locked by another process")]
    Locked,
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Invalid configuration value for '{key}': {value}")]
    InvalidValue { key: String, value: String },
    
    #[error("Missing required configuration: {key}")]
    Missing { key: String },
    
    #[error("Configuration file format error")]
    Format(#[from] toml::de::Error),
}

#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Invalid passphrase")]
    InvalidPassphrase,
    
    #[error("Encryption failed: corrupted data")]
    CorruptedData,
    
    #[error("Key derivation failed")]
    KeyDerivation,
}

// Result type alias
pub type Result<T> = std::result::Result<T, WhiteoutError>;

// Conversion helpers
impl From<std::io::Error> for WhiteoutError {
    fn from(err: std::io::Error) -> Self {
        WhiteoutError::Io {
            operation: "file operation".to_string(),
            source: err,
        }
    }
}
```

### 4. Error Context and Recovery

**Implementation**:

```rust
// src/transform/smudge.rs - Enhanced error handling
use crate::error::{Result, WhiteoutError, StorageError};

pub fn apply(
    content: &str,
    file_path: &Path,
    storage: &LocalStorage,
    _config: &Config,
) -> Result<String> {
    let parser = Parser::new();
    let mut decorations = parser.parse(content)?;
    
    if decorations.is_empty() {
        return Ok(content.to_string());
    }
    
    // Enhanced error handling with recovery
    for decoration in &mut decorations {
        match decoration {
            Decoration::Inline { line, local_value, .. } => {
                match storage.get_value(file_path, &format!("inline_{}", line)) {
                    Ok(stored_value) => *local_value = Arc::from(stored_value),
                    Err(StorageError::KeyNotFound { key }) => {
                        tracing::warn!(
                            "Local value not found for key '{}' in file '{}', using fallback",
                            key, file_path.display()
                        );
                        // Keep original value as fallback - graceful degradation
                    }
                    Err(StorageError::Corrupted { reason }) => {
                        return Err(WhiteoutError::Storage(StorageError::Corrupted { 
                            reason: format!("Storage corrupted while processing {}: {}", file_path.display(), reason)
                        }));
                    }
                    Err(e) => return Err(WhiteoutError::Storage(e)),
                }
            }
            // Similar pattern for other decoration types...
        }
    }
    
    Ok(parser.apply_decorations(content, &decorations, true))
}
```

## Priority 3: Performance Optimizations

### 5. Streaming Line Processing

**Implementation**:

```rust
// src/parser/mod.rs - Streaming apply_decorations
use std::io::Write;

impl Parser {
    pub fn apply_decorations(&self, content: &str, decorations: &[Decoration], use_local: bool) -> String {
        // Pre-allocate with estimated capacity
        let estimated_capacity = if use_local {
            content.len() * 2 // Local version typically larger
        } else {
            content.len() // Cleaned version typically same size
        };
        
        let mut result = String::with_capacity(estimated_capacity);
        let mut line_number = 1u32;
        let mut skip_until = 0u32;
        
        // Create decoration lookup maps for O(1) access
        let inline_map: HashMap<u32, &Decoration> = decorations.iter()
            .filter_map(|d| match d {
                Decoration::Inline { line, .. } => Some((*line, d)),
                _ => None,
            })
            .collect();
            
        let block_map: HashMap<u32, &Decoration> = decorations.iter()
            .filter_map(|d| match d {
                Decoration::Block { start_line, .. } => Some((*start_line, d)),
                _ => None,
            })
            .collect();
        
        // Single-pass processing
        for line in content.lines() {
            if line_number <= skip_until {
                line_number += 1;
                continue;
            }
            
            // Check for decorations - O(1) lookup instead of O(n) iteration
            if let Some(decoration) = block_map.get(&line_number) {
                skip_until = self.process_block_decoration(decoration, use_local, &mut result);
            } else if let Some(decoration) = inline_map.get(&line_number) {
                self.process_inline_decoration(decoration, use_local, &mut result);
            } else {
                // Check for partial decorations
                self.process_line_with_partials(line, line_number, decorations, use_local, &mut result);
            }
            
            line_number += 1;
        }
        
        result
    }
    
    fn process_inline_decoration(&self, decoration: &Decoration, use_local: bool, result: &mut String) {
        match decoration {
            Decoration::Inline { local_value, committed_value, .. } => {
                if use_local {
                    write!(result, "{} // @whiteout: \"{}\"\n", local_value, committed_value).unwrap();
                } else {
                    writeln!(result, "{}", committed_value).unwrap();
                }
            }
            _ => unreachable!(),
        }
    }
}
```

### 6. Atomic File Operations

**Implementation**:

```rust
// src/storage/local.rs - Thread-safe file operations
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write, Read};
use fs2::FileExt;

impl LocalStorage {
    pub fn store_value(&self, file_path: &Path, key: &str, value: &str) -> Result<()> {
        let storage_key = self.make_storage_key(file_path, key);
        
        let entry = StorageEntry {
            file_path: file_path.to_path_buf(),
            key: key.to_string(),
            value: value.to_string(),
            encrypted: false,
            timestamp: chrono::Utc::now(),
        };
        
        // Atomic update with file locking
        self.atomic_update(|data| {
            data.entries.insert(storage_key, entry);
            Ok(())
        })
    }
    
    fn atomic_update<F>(&self, update_fn: F) -> Result<()> 
    where
        F: FnOnce(&mut StorageData) -> Result<()>,
    {
        // Ensure parent directory exists
        if let Some(parent) = self.storage_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| WhiteoutError::Io {
                    operation: format!("create directory {}", parent.display()),
                    source: e,
                })?;
        }
        
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&self.storage_path)
            .map_err(|e| WhiteoutError::Io {
                operation: format!("open storage file {}", self.storage_path.display()),
                source: e,
            })?;
        
        // Exclusive lock - blocks until available
        file.lock_exclusive()
            .map_err(|e| WhiteoutError::Storage(StorageError::Locked))?;
        
        // Read current data
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        
        let mut data: StorageData = if content.is_empty() {
            StorageData::default()
        } else {
            toml::from_str(&content)
                .map_err(|_| StorageError::Corrupted { 
                    reason: "Invalid TOML format".to_string() 
                })?
        };
        
        // Apply update
        update_fn(&mut data)?;
        
        // Write atomically
        let new_content = toml::to_string_pretty(&data)
            .map_err(|e| WhiteoutError::Config(ConfigError::Format(e)))?;
        
        file.seek(SeekFrom::Start(0))?;
        file.set_len(0)?; // Truncate
        file.write_all(new_content.as_bytes())?;
        file.flush()?;
        
        // Lock automatically released when file is dropped
        Ok(())
    }
}
```

## Priority 4: API Ergonomics

### 7. Builder Pattern Implementation

**Implementation**:

```rust
// src/lib.rs - Enhanced with builder pattern
use crate::error::Result;

#[derive(Default)]
pub struct WhiteoutBuilder {
    project_root: Option<PathBuf>,
    custom_config: Option<ConfigData>,
    encryption_passphrase: Option<String>,
    string_interner: Option<StringInterner>,
    performance_mode: bool,
}

impl WhiteoutBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn project_root<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.project_root = Some(path.as_ref().to_path_buf());
        self
    }
    
    pub fn encryption_passphrase<S: Into<String>>(mut self, passphrase: S) -> Self {
        self.encryption_passphrase = Some(passphrase.into());
        self
    }
    
    pub fn custom_patterns(mut self, patterns: DecorationPatterns) -> Self {
        let mut config = self.custom_config.unwrap_or_default();
        config.decorations = patterns.into();
        self.custom_config = Some(config);
        self
    }
    
    pub fn performance_mode(mut self, enabled: bool) -> Self {
        self.performance_mode = enabled;
        self
    }
    
    pub fn build(self) -> Result<Whiteout> {
        let project_root = self.project_root
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        
        let config = if let Some(custom) = self.custom_config {
            Config { data: custom, path: project_root.join(".whiteout/config.toml") }
        } else {
            Config::load_or_default(&project_root)?
        };
        
        let storage = if self.performance_mode {
            LocalStorage::new_with_caching(&project_root)?
        } else {
            LocalStorage::new(&project_root)?
        };
        
        let interner = self.string_interner.unwrap_or_else(StringInterner::new);
        
        Ok(Whiteout {
            config: Arc::new(config),
            storage: Arc::new(storage),
            interner: Arc::new(interner),
        })
    }
}

impl Whiteout {
    pub fn builder() -> WhiteoutBuilder {
        WhiteoutBuilder::new()
    }
    
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        Self::builder().project_root(project_root).build()
    }
}

// Usage examples:
// let whiteout = Whiteout::builder()
//     .project_root(".")
//     .encryption_passphrase("secret")
//     .performance_mode(true)
//     .build()?;
```

### 8. Async API Surface

**Implementation**:

```rust
// src/lib.rs - Async support
use tokio::task;

impl Whiteout {
    pub async fn clean_async(&self, content: &str, file_path: &Path) -> Result<String> {
        let this = Arc::clone(self);
        let content = content.to_string();
        let file_path = file_path.to_path_buf();
        
        task::spawn_blocking(move || {
            this.clean(&content, &file_path)
        })
        .await
        .map_err(|e| WhiteoutError::Git { 
            message: format!("Task execution failed: {}", e) 
        })?
    }
    
    pub async fn smudge_async(&self, content: &str, file_path: &Path) -> Result<String> {
        let this = Arc::clone(self);
        let content = content.to_string();
        let file_path = file_path.to_path_buf();
        
        task::spawn_blocking(move || {
            this.smudge(&content, &file_path)
        })
        .await
        .map_err(|e| WhiteoutError::Git { 
            message: format!("Task execution failed: {}", e) 
        })?
    }
    
    pub async fn batch_process_async<P>(&self, files: Vec<P>) -> Result<Vec<ProcessResult>>
    where
        P: AsRef<Path> + Send + 'static,
    {
        let futures = files.into_iter().map(|file_path| {
            let this = Arc::clone(self);
            let file_path = file_path.as_ref().to_path_buf();
            
            async move {
                let content = tokio::fs::read_to_string(&file_path).await?;
                let cleaned = this.clean_async(&content, &file_path).await?;
                Ok(ProcessResult { file_path, content: cleaned })
            }
        });
        
        futures::future::try_join_all(futures).await
    }
}

#[derive(Debug)]
pub struct ProcessResult {
    pub file_path: PathBuf,
    pub content: String,
}
```

## Priority 5: Testing Enhancements

### 9. Property-Based Testing

**Implementation**:

```rust
// tests/property_tests.rs - New file
use proptest::prelude::*;
use whiteout::{Whiteout, parser::Parser};

prop_compose! {
    fn arb_decoration_content()(
        local_value in r"[a-zA-Z0-9_]{1,50}",
        committed_value in r"[a-zA-Z0-9_]{1,50}"
    ) -> (String, String) {
        (local_value, committed_value)
    }
}

prop_compose! {
    fn arb_inline_decoration()(
        (local, committed) in arb_decoration_content(),
        prefix in r"[a-zA-Z_][a-zA-Z0-9_]*\s*=\s*",
        comment_style in prop_oneof!["//", "#", "--"]
    ) -> String {
        format!("{}\"{}\"; {} @whiteout: \"{}\"", prefix, local, comment_style, committed)
    }
}

proptest! {
    #[test]
    fn test_parse_apply_roundtrip(content in arb_inline_decoration()) {
        let parser = Parser::new();
        
        // Property: parsing then applying should be deterministic
        if let Ok(decorations) = parser.parse(&content) {
            let cleaned1 = parser.apply_decorations(&content, &decorations, false);
            let cleaned2 = parser.apply_decorations(&content, &decorations, false);
            
            prop_assert_eq!(cleaned1, cleaned2, "Clean operation should be deterministic");
            
            // Property: clean should not contain local values
            for decoration in &decorations {
                match decoration {
                    whiteout::parser::Decoration::Inline { local_value, .. } => {
                        prop_assert!(
                            !cleaned1.contains(local_value),
                            "Cleaned content should not contain local value: {}", 
                            local_value
                        );
                    }
                    _ => {}
                }
            }
        }
    }
    
    #[test]
    fn test_storage_key_uniqueness(
        paths in prop::collection::vec(r"[a-zA-Z0-9./]{1,100}", 1..10),
        keys in prop::collection::vec(r"[a-zA-Z0-9_]{1,50}", 1..10)
    ) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let storage = whiteout::storage::LocalStorage::new(temp_dir.path()).unwrap();
        
        let mut generated_keys = std::collections::HashSet::new();
        
        for path_str in paths {
            for key in &keys {
                let path = std::path::Path::new(&path_str);
                let storage_key = storage.make_storage_key(path, key);
                
                prop_assert!(
                    generated_keys.insert(storage_key.clone()),
                    "Storage key collision detected: {}",
                    storage_key
                );
            }
        }
    }
}
```

### 10. Benchmark-Driven Optimization

**Implementation**:

```rust
// benches/comprehensive_benchmarks.rs
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use whiteout::{Whiteout, parser::Parser};
use std::hint::black_box;

fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    
    for size in [1_000, 10_000, 100_000].iter() {
        let content = generate_large_file_content(*size);
        
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("parse_decorations", size),
            &content,
            |b, content| {
                let parser = Parser::new();
                b.iter(|| {
                    black_box(parser.parse(black_box(content)).unwrap())
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("apply_decorations", size),
            &content,
            |b, content| {
                let parser = Parser::new();
                let decorations = parser.parse(content).unwrap();
                b.iter(|| {
                    black_box(parser.apply_decorations(
                        black_box(content),
                        black_box(&decorations),
                        black_box(false)
                    ))
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_concurrent_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_storage");
    
    for thread_count in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_writes", thread_count),
            thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let temp_dir = tempfile::TempDir::new().unwrap();
                    let storage = std::sync::Arc::new(
                        whiteout::storage::LocalStorage::new(temp_dir.path()).unwrap()
                    );
                    
                    let handles: Vec<_> = (0..thread_count)
                        .map(|i| {
                            let storage = std::sync::Arc::clone(&storage);
                            std::thread::spawn(move || {
                                for j in 0..100 {
                                    storage.store_value(
                                        std::path::Path::new(&format!("file_{}.rs", i)),
                                        &format!("key_{}", j),
                                        &format!("value_{}_{}", i, j),
                                    ).unwrap();
                                }
                            })
                        })
                        .collect();
                    
                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(benches, benchmark_memory_usage, benchmark_concurrent_access);
criterion_main!(benches);
```

## Implementation Timeline

### Week 1: Memory & Performance
- [ ] Implement optimized enum layouts
- [ ] Add string interning system
- [ ] Optimize line processing algorithms
- [ ] Add comprehensive benchmarks

### Week 2: Error Handling & Threading
- [ ] Replace anyhow with structured errors
- [ ] Implement atomic file operations  
- [ ] Add thread safety guarantees
- [ ] Implement graceful error recovery

### Week 3: API Design
- [ ] Add builder pattern
- [ ] Implement async API surface
- [ ] Add comprehensive documentation
- [ ] Implement trait derivations

### Week 4: Testing & Validation
- [ ] Add property-based tests
- [ ] Implement fuzz testing
- [ ] Add integration benchmarks
- [ ] Performance regression testing

This implementation guide provides the concrete code needed to transform Whiteout into a production-ready, high-performance tool while maintaining backward compatibility and improving developer experience.