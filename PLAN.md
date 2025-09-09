# Whiteout: Local-Only Code Decoration Tool

## Project Overview

**Whiteout** is a Rust-based developer tool that allows marking specific parts of code as "local-only", preventing them from being committed to Git while maintaining local functionality. This solves the common problem of accidentally committing secrets, debug code, or local configurations.

## Problem Statement

Developers often need to:
- Keep secrets/API keys locally while committing placeholder code
- Maintain local debug statements without polluting commits
- Use local development configurations that differ from production
- Test with hardcoded values that shouldn't be in version control

Current solutions (gitignore, environment variables) are insufficient for inline code changes.

## Technical Approach

### Architecture: Hybrid Git Integration

**Recommended Approach**: Combination of Git filters and CLI tool

1. **Git Clean/Smudge Filters**: Core transformation mechanism
   - Clean filter: Transforms decorated code before commit
   - Smudge filter: Restores local values after checkout
   
2. **Pre-commit Hook**: Validation and safety checks
   - Ensures no sensitive data escapes
   - Validates transformation syntax
   
3. **CLI Tool**: User interface and management
   - Configure decorations
   - Manage local value storage
   - Initialize projects

### How It Works

```
Working Directory → [Smudge Filter] → Index → [Clean Filter] → Repository
     (local)           (restore)              (sanitize)         (safe)
```

## Core Features

### 1. Decoration Syntax

Multiple decoration formats for flexibility:

#### Inline Decoration
```rust
// @whiteout-start
let api_key = "sk-1234567890abcdef";  // Local only
// @whiteout-end
let api_key = load_from_env();        // Committed version
```

#### Single Line
```rust
let api_key = "sk-1234567890abcdef"; // @whiteout: load_from_env()
```

#### Partial Line
```rust
let url = "https://[[dev.api.local||api.example.com]]/v1";
```

### 2. Local Value Storage

- Store local values in `.whiteout/local.toml` (gitignored)
- Optional encryption for sensitive data
- Per-branch configurations supported

### 3. Language Agnostic

Initial support for:
- Rust
- JavaScript/TypeScript
- Python
- Go
- Configuration files (JSON, YAML, TOML)

## Implementation Plan

### Phase 1: Core Library (Week 1-2)

**Tasks:**
1. Create Rust project structure
2. Implement decoration parser
3. Build transformation engine
4. Create local storage system
5. Add encryption support

**File Structure:**
```
whiteout/
├── Cargo.toml
├── src/
│   ├── main.rs           # CLI entry point
│   ├── lib.rs            # Library exports
│   ├── parser/
│   │   ├── mod.rs
│   │   ├── inline.rs     # Inline decoration parser
│   │   ├── block.rs      # Block decoration parser
│   │   └── partial.rs    # Partial line parser
│   ├── transform/
│   │   ├── mod.rs
│   │   ├── clean.rs      # Git clean filter
│   │   └── smudge.rs     # Git smudge filter
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── local.rs      # Local value storage
│   │   └── crypto.rs     # Encryption utilities
│   └── config/
│       ├── mod.rs
│       └── project.rs    # Project configuration
```

### Phase 2: Git Integration (Week 2-3)

**Tasks:**
1. Implement Git clean filter
2. Implement Git smudge filter
3. Create pre-commit hook
4. Build Git configuration helpers
5. Add diff driver for better diffs

**Git Configuration:**
```gitattributes
* filter=whiteout
```

```gitconfig
[filter "whiteout"]
    clean = whiteout clean
    smudge = whiteout smudge
    required = true
```

### Phase 3: CLI Tool (Week 3-4)

**Tasks:**
1. Build CLI using clap
2. Implement init command
3. Add mark/unmark commands
4. Create status command
5. Build config management

**CLI Commands:**
```bash
whiteout init                      # Initialize in project
whiteout mark <file:line>          # Mark line as local-only
whiteout unmark <file:line>        # Remove decoration
whiteout status                     # Show decorated files
whiteout config set <key> <value>  # Configure tool
whiteout sync                       # Sync local values
```

### Phase 4: Testing & Documentation (Week 4-5)

**Tasks:**
1. Unit tests for parser
2. Integration tests for Git filters
3. End-to-end CLI tests
4. Performance benchmarks
5. Write comprehensive documentation
6. Create example projects

### Phase 5: Advanced Features (Week 5-6)

**Tasks:**
1. IDE extensions (VS Code first)
2. Team collaboration features
3. Migration tools from other solutions
4. GitHub Actions integration
5. Pre-commit framework integration

## Example Usage Scenarios

### Scenario 1: API Keys
```javascript
// Before decoration
const apiKey = process.env.API_KEY;

// After decoration (local)
const apiKey = "sk-myactualapikey123"; // @whiteout: process.env.API_KEY

// In repository
const apiKey = process.env.API_KEY;
```

### Scenario 2: Debug Configuration
```rust
// @whiteout-start
const DEBUG: bool = true;
const LOG_LEVEL: &str = "trace";
// @whiteout-end
const DEBUG: bool = false;
const LOG_LEVEL: &str = "error";
```

### Scenario 3: Development URLs
```python
# Using partial decoration
api_url = "https://[[localhost:8080||api.production.com]]/v1"
```

## Technical Challenges & Solutions

### Challenge 1: Merge Conflicts
**Solution**: Custom merge driver that understands decorations and preserves local values during merges.

### Challenge 2: Performance
**Solution**: 
- Lazy loading of decorations
- Caching parsed results
- Parallel processing for large files

### Challenge 3: Accidental Commits
**Solution**:
- Pre-commit hook validation
- CI/CD integration to catch escapes
- Audit logging of transformations

### Challenge 4: Team Collaboration
**Solution**:
- Shared decoration definitions
- Template system for common patterns
- Team configuration profiles

## Security Considerations

1. **Encryption**: AES-256 for local value storage
2. **Access Control**: File permissions for `.whiteout/` directory
3. **Audit Trail**: Log all transformations
4. **Validation**: Strict parsing to prevent injection
5. **Secret Scanning**: Integration with secret scanning tools

## Future Enhancements

1. **Cloud Sync**: Optional encrypted cloud storage for team sharing
2. **Smart Detection**: ML-based detection of potential secrets
3. **Git Hooks Manager**: Comprehensive hook management beyond pre-commit
4. **Language Servers**: LSP implementation for real-time feedback
5. **Web UI**: Browser-based configuration interface

## Success Metrics

- Zero accidental secret commits
- < 10ms transformation time for average file
- Seamless Git workflow integration
- Support for 90% of common use cases
- Active community adoption

## MVP Definition

The Minimum Viable Product includes:
1. Core transformation engine
2. Git filter integration
3. Basic CLI (init, mark, status)
4. Support for 3 languages (Rust, JS, Python)
5. Local storage without encryption
6. Basic documentation

## Timeline

- **Week 1-2**: Core library and parser
- **Week 2-3**: Git integration
- **Week 3-4**: CLI development
- **Week 4-5**: Testing and documentation
- **Week 5-6**: Polish and advanced features
- **Week 6+**: Community feedback and iteration

## Conclusion

Whiteout provides a robust solution for managing local-only code changes while maintaining clean Git history. By leveraging Git's filter system and providing intuitive decorations, it seamlessly integrates into existing workflows while preventing accidental exposure of sensitive information.