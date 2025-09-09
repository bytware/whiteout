# Whiteout Performance Audit - Second Pass

## Executive Summary

After the initial optimization pass that achieved 78% improvement through static regex compilation, this second audit identifies deeper optimization opportunities focusing on memory allocation, algorithmic complexity, and system-level improvements.

## Current Performance Baseline

- **Binary Size**: 2.6MB (release build with LTO)
- **Small File (100 lines)**: ~11ms
- **Medium File (1K lines)**: ~12ms  
- **Memory Usage**: ~6.9MB resident set size
- **Previous Optimization**: Static regex compilation (78% improvement)

## Critical Performance Issues Identified

### 1. **O(n*m) Complexity in apply_decorations** [HIGH PRIORITY]

**Location**: `src/parser/mod.rs:70-187`

**Issue**: The `apply_decorations` method has nested loops iterating over all decorations for each line, resulting in O(n*m) complexity where n = lines and m = decorations.

```rust
for (idx, line) in lines.iter().enumerate() {
    // Checking ALL decorations for EACH line
    for decoration in decorations {
        if let Decoration::Block { ... } = decoration {
            if line_num == *start_line { ... }
        }
    }
    // This pattern repeats for inline and partial decorations
}
```

**Impact**: For a 10K line file with 100 decorations, this results in 1M comparisons.

**Solution**:
```rust
// Pre-index decorations by line number
let mut decoration_map: HashMap<usize, Vec<&Decoration>> = HashMap::new();
for decoration in decorations {
    match decoration {
        Decoration::Inline { line, .. } => {
            decoration_map.entry(*line).or_default().push(decoration);
        }
        Decoration::Block { start_line, .. } => {
            decoration_map.entry(*start_line).or_default().push(decoration);
        }
        Decoration::Partial { line, .. } => {
            decoration_map.entry(*line).or_default().push(decoration);
        }
    }
}

// Now O(1) lookup per line
for (idx, line) in lines.iter().enumerate() {
    if let Some(line_decorations) = decoration_map.get(&(idx + 1)) {
        // Process only relevant decorations
    }
}
```

**Expected Improvement**: 90%+ reduction in processing time for large files.

### 2. **Excessive String Allocations** [HIGH PRIORITY]

**Location**: Multiple locations

**Issues**:
- `to_string()` called unnecessarily in hot paths
- String concatenation in loops without pre-allocation
- No string interning for repeated values

**Examples**:
```rust
// src/parser/mod.rs:182
result.push(line.to_string());  // Allocates even when unchanged

// src/parser/inline.rs:37-44
local_value: local_value.trim().to_string(),  // Double allocation
committed_value: committed_value.trim().to_string(),
```

**Solution**:
```rust
// Use Cow<str> to avoid allocations when possible
use std::borrow::Cow;

// Pre-allocate with capacity
let mut result = Vec::with_capacity(lines.len());

// Use string references when possible
match line_needs_modification {
    true => result.push(Cow::Owned(modified_line)),
    false => result.push(Cow::Borrowed(line)),
}
```

**Expected Improvement**: 30-40% reduction in memory allocations.

### 3. **Inefficient File I/O** [MEDIUM PRIORITY]

**Location**: `src/storage/local.rs`

**Issue**: The storage system reads and writes the entire TOML file for every operation.

```rust
// Every store_value call:
1. Read entire file
2. Parse TOML
3. Modify one entry
4. Serialize entire structure
5. Write entire file
```

**Solution**:
```rust
// Implement write batching
pub struct BatchedStorage {
    pending_writes: HashMap<String, StorageEntry>,
    last_flush: Instant,
}

impl BatchedStorage {
    pub fn store_value(&mut self, key: String, entry: StorageEntry) {
        self.pending_writes.insert(key, entry);
        if self.pending_writes.len() > 100 || 
           self.last_flush.elapsed() > Duration::from_secs(1) {
            self.flush();
        }
    }
}
```

**Expected Improvement**: 80% reduction in I/O operations for batch operations.

### 4. **Missing Parallelization Opportunities** [MEDIUM PRIORITY]

**Location**: Parser processing

**Issue**: Files are processed sequentially despite decorations being independent.

**Solution**:
```rust
use rayon::prelude::*;

// Parallel parsing
let decorations: Vec<Decoration> = content
    .par_lines()
    .enumerate()
    .flat_map(|(line_num, line)| {
        let mut local_decorations = Vec::new();
        // Parse inline, block, partial in parallel
        local_decorations
    })
    .collect();
```

**Expected Improvement**: 2-4x speedup on multi-core systems.

### 5. **Regex Pattern Matching Inefficiencies** [LOW PRIORITY]

**Issue**: Even with static compilation, regex matching could be optimized.

**Solution**:
```rust
// Use aho-corasick for multiple pattern matching
use aho_corasick::AhoCorasick;

static PATTERNS: Lazy<AhoCorasick> = Lazy::new(|| {
    AhoCorasick::new(&[
        "@whiteout:",
        "@whiteout-start",
        "@whiteout-end",
        "[[", "||", "]]"
    ])
});

// Fast pre-filter before regex
if !PATTERNS.is_match(line) {
    continue; // Skip regex entirely
}
```

**Expected Improvement**: 10-20% for files with few decorations.

## Memory Optimization Opportunities

### 1. **Zero-Copy Operations**

Replace string copies with slices and indices:
```rust
pub struct Decoration {
    line_range: Range<usize>,  // Instead of storing content
    replacement_indices: Vec<(usize, usize)>,
}
```

### 2. **Arena Allocation**

Use arena allocator for temporary strings:
```rust
use bumpalo::Bump;

let arena = Bump::new();
let temp_string = arena.alloc_str("temporary");
```

### 3. **String Deduplication**

Intern common strings:
```rust
use string_cache::DefaultAtom;

static REDACTED: DefaultAtom = DefaultAtom::from("REDACTED");
```

## Benchmark Improvements Needed

1. **Add micro-benchmarks** for individual components
2. **Profile with flamegraph** to identify hot spots
3. **Add memory usage benchmarks**
4. **Test with realistic repository sizes** (100K+ lines)
5. **Benchmark Git filter integration** end-to-end

## Implementation Priority

1. **Fix O(n*m) complexity** - Highest impact, easiest to implement
2. **Reduce string allocations** - High impact on memory usage
3. **Batch storage I/O** - Important for large repositories
4. **Add parallelization** - Good speedup for multi-file operations
5. **Optimize regex matching** - Minor improvements

## Performance Targets

- **10K line file**: < 50ms (currently ~100ms estimated)
- **Memory usage**: < 2x file size (currently ~3x)
- **100K line repository**: < 1 second full scan
- **Git filter overhead**: < 10ms for unchanged files

## Next Steps

1. Implement decoration indexing (1-2 hours)
2. Add Cow<str> for string handling (2-3 hours)
3. Implement storage batching (1-2 hours)
4. Add Rayon parallelization (2-3 hours)
5. Create comprehensive benchmark suite (2-3 hours)

## Estimated Total Impact

With all optimizations implemented:
- **70-90% reduction** in processing time for large files
- **50% reduction** in memory usage
- **4x improvement** for multi-file operations on multi-core systems
- **Near-zero overhead** for files without decorations