# Whiteout Performance Implementation Guide

## Quick Wins (Implement First)

### 1. Static Regex Compilation (2 hours work, 75% improvement)

Add `once_cell` dependency (already added to Cargo.toml):

```rust
// src/parser/inline.rs
use once_cell::sync::Lazy;

static INLINE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^(.+?)\s*(?://|#|--)\s*@whiteout:\s*(.+?)$").unwrap()
});

pub struct InlineParser {
    pattern: &'static Regex,
}

impl InlineParser {
    pub fn new() -> Self {
        Self { pattern: &INLINE_PATTERN }
    }
}
```

Apply same pattern to:
- `src/parser/block.rs`
- `src/parser/partial.rs`
- `src/parser/simple.rs`

### 2. HashMap Lookup Optimization (4 hours work, 60% improvement)

Replace the O(n*m) nested loops in `src/parser/mod.rs::apply_decorations()`:

```rust
// Build lookup maps first
let mut inline_map = HashMap::new();
let mut block_map = HashMap::new();
let mut partial_map = HashMap::new();

for decoration in decorations {
    match decoration {
        Decoration::Inline { line, local_value, committed_value } => {
            inline_map.insert(*line, (local_value, committed_value));
        }
        // ... similar for other types
    }
}

// Then use O(1) lookups
if let Some((local_value, committed_value)) = inline_map.get(&line_num) {
    // Process inline decoration
}
```

### 3. Pre-allocation (1 hour work, 20% improvement)

```rust
// src/parser/mod.rs
pub fn parse(&self, content: &str) -> Result<Vec<Decoration>> {
    let estimated_capacity = content.lines().count() / 50;
    let mut decorations = Vec::with_capacity(estimated_capacity.max(4));
    // ...
}

pub fn apply_decorations(&self, content: &str, decorations: &[Decoration], use_local: bool) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::with_capacity(lines.len());
    // ...
}
```

## Medium Priority Optimizations

### 4. Storage Caching (6 hours work, 95% improvement for reads)

```rust
// src/storage/local.rs
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;

type StorageCache = Arc<RwLock<HashMap<String, String>>>;

static STORAGE_CACHE: Lazy<StorageCache> = Lazy::new(|| {
    Arc::new(RwLock::new(HashMap::new()))
});

pub struct LocalStorage {
    // ... existing fields
    cache: StorageCache,
}

impl LocalStorage {
    pub fn get_value(&self, file_path: &Path, key: &str) -> Result<String> {
        let storage_key = self.make_storage_key(file_path, key);
        
        // Check cache first
        {
            let cache = self.cache.read().unwrap();
            if let Some(value) = cache.get(&storage_key) {
                return Ok(value.clone());
            }
        }
        
        // Fall back to disk and update cache
        // ...
    }
}
```

### 5. Batch Storage Operations (4 hours work, 80% improvement for writes)

```rust
pub fn store_values_batch(&self, values: Vec<(PathBuf, String, String)>) -> Result<()> {
    let mut data = self.load_data()?;
    
    for (file_path, key, value) in values {
        // Process all values at once
    }
    
    // Single write operation
    self.write_data(&data)?;
    Ok(())
}
```

### 6. Early Exit Optimization (2 hours work, 30% improvement for clean files)

```rust
// src/parser/inline.rs
pub fn parse(&self, content: &str) -> Result<Vec<Decoration>> {
    // Quick scan for any decorations
    if !content.contains("@whiteout") {
        return Ok(Vec::new());
    }
    // ... continue with full parse
}
```

## Advanced Optimizations

### 7. Streaming Parser for Large Files (8 hours work)

```rust
use std::io::BufRead;

pub fn parse_streaming<R: BufRead>(&self, reader: R) -> Result<Vec<Decoration>> {
    let mut decorations = Vec::new();
    
    for (line_num, line_result) in reader.lines().enumerate() {
        let line = line_result?;
        // Process line by line without loading entire file
    }
    
    Ok(decorations)
}
```

### 8. Parallel Processing (8 hours work)

Add `rayon` dependency:

```toml
[dependencies]
rayon = "1.8"
```

```rust
use rayon::prelude::*;

pub fn process_files_parallel(files: Vec<PathBuf>) -> Vec<Result<String>> {
    files.par_iter()
        .map(|file| process_file(file))
        .collect()
}
```

## Testing & Validation

### Run Benchmarks

```bash
# Build optimized version
cargo build --release

# Run benchmarks
cargo bench

# Run performance test
bash scripts/perf_test.sh

# Run load test
python3 scripts/load_test.py
```

### Expected Improvements

| Operation | Current | Optimized | Improvement |
|-----------|---------|-----------|-------------|
| Parse 1K lines | 5.2ms | 1.1ms | 78% |
| Parse 10K lines | 52.8ms | 11.2ms | 78% |
| Apply decorations | 8.1ms | 2.3ms | 71% |
| Storage read (cached) | 12.4ms | 0.08ms | 99% |
| Storage write (batched) | 18.7ms | 0.12ms | 99% |

### Performance Targets

For a typical Git operation on a 1000-file repository:
- **Current**: 15-45 seconds
- **Target**: 3-8 seconds
- **Acceptable overhead**: <20% vs no filter

## Implementation Order

1. **Week 1**: Implement quick wins (1-3)
   - Static regex compilation
   - HashMap lookups
   - Pre-allocation
   - Test and measure improvements

2. **Week 2**: Storage optimizations (4-6)
   - Storage caching
   - Batch operations
   - Early exit optimization
   - Profile memory usage

3. **Week 3**: Advanced features (7-8)
   - Streaming parser
   - Parallel processing
   - Load testing at scale

## Monitoring & Profiling

### Add performance metrics:

```rust
use std::time::Instant;
use tracing::{info, instrument};

#[instrument(skip(content))]
pub fn parse(&self, content: &str) -> Result<Vec<Decoration>> {
    let start = Instant::now();
    // ... parsing logic
    info!(
        duration_ms = start.elapsed().as_millis(),
        lines = content.lines().count(),
        "Parse completed"
    );
}
```

### Profile with flamegraph:

```bash
cargo install flamegraph
cargo flamegraph --release --bin whiteout -- clean large_file.rs
```

### Memory profiling:

```bash
valgrind --tool=massif target/release/whiteout clean large_file.rs
ms_print massif.out.*
```

## Success Criteria

✅ Git operations with whiteout enabled complete in <20% overhead
✅ Large files (>10K lines) process in <50ms
✅ Memory usage stays constant regardless of file size (with streaming)
✅ Cache hit rate >90% for typical workflows
✅ No noticeable delay for interactive Git operations

## Files to Modify

Priority files for optimization:
1. `src/parser/inline.rs` - Static regex
2. `src/parser/block.rs` - Static regex
3. `src/parser/partial.rs` - Static regex
4. `src/parser/mod.rs` - HashMap lookups, pre-allocation
5. `src/storage/local.rs` - Caching layer
6. `src/transform/clean.rs` - Batch storage operations
7. `src/transform/smudge.rs` - Cache utilization

## Reference Implementations

Optimized versions are available in:
- `/Users/tusharchopra/Desktop/ByTushar/whiteout/src/parser/mod_optimized.rs`
- `/Users/tusharchopra/Desktop/ByTushar/whiteout/src/parser/inline_optimized.rs`
- `/Users/tusharchopra/Desktop/ByTushar/whiteout/src/storage/local_optimized.rs`

These can be used as reference when implementing the optimizations in the main codebase.