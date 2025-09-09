# Whiteout - Improved Design

## Key Improvements Made

### 1. ✅ Fixed Critical Security Bug
- **Problem**: Clean filter was returning original content with secrets
- **Solution**: Now properly transforms content to remove local values before committing

### 2. ✅ Automatic Git Configuration
- **Problem**: Users had to manually configure Git filters after init
- **Solution**: `whiteout init` now automatically configures everything

### 3. ✅ Preview Command
- **Problem**: Users couldn't see what would be committed vs what stays local
- **Solution**: Added `whiteout preview <file>` with optional `--diff` flag

### 4. ✅ Security Checking
- **Problem**: No way to detect undecorated secrets
- **Solution**: Added `whiteout check` command with `--fix` option

## Improved Decoration Syntax Options

### Current Syntax (Still Supported)

#### Inline
```javascript
let apiKey = "sk-12345"; // @whiteout: process.env.API_KEY
```

#### Block (Verbose - requires duplication)
```rust
// @whiteout-start
const DEBUG = true;
// @whiteout-end
const DEBUG = false;
```

### NEW: Simplified Syntax (Better Ergonomics)

#### Simple Redaction
```javascript
let apiKey = "sk-12345"; // @whiteout
// Automatically replaced with "REDACTED" in commits
```

#### Auto-Environment Variable
```javascript
let apiKey = "sk-12345"; // @whiteout-env: API_KEY
// Automatically replaced with process.env.API_KEY or equivalent
```

#### Single Block (No Duplication)
```rust
// @whiteout-block: production
const DEBUG = true;
const LOG_LEVEL = "trace";
// @whiteout-end
// Entire block removed in commits, replaced with comment: "// [Production config hidden]"
```

## Better Error Messages

### Before
```
Error: Failed to parse storage file
```

### After
```
Error: Failed to parse .whiteout/local.toml
  Reason: Invalid TOML syntax at line 5
  Fix: Check for missing quotes or commas
  Help: Run 'whiteout repair' to attempt automatic fix
```

## Safety Features

1. **Pre-commit validation**: Warns if committing files with potential secrets
2. **Dry-run mode**: Test decorations without affecting files
3. **Backup before changes**: Auto-backup when using mark/unmark
4. **Recovery command**: `whiteout recover` to restore from backups

## Workflow Improvements

### Quick Start (One Command)
```bash
whiteout init --check
# Automatically:
# 1. Initializes whiteout
# 2. Configures Git
# 3. Scans for potential secrets
# 4. Offers to add decorations
```

### CI/CD Integration
```yaml
- name: Check for secrets
  run: whiteout check --ci
  # Fails build if undecorated secrets found
```

## Performance Optimizations

1. **Parallel processing**: Process multiple files concurrently
2. **Incremental updates**: Only reprocess changed files
3. **Caching**: Cache parsed decorations for faster operations

## User Experience Polish

1. **Interactive mode**: `whiteout mark -i` for guided decoration
2. **Undo support**: `whiteout undo` to revert last operation
3. **Status bar**: Show progress for long operations
4. **Help context**: `whiteout help <command>` for detailed examples

## Migration Path

For users already using other secret management:
```bash
whiteout migrate --from=git-secrets
whiteout migrate --from=dotenv
```

## Summary

The improved design focuses on:
- **Safety**: Preventing accidental secret commits
- **Ergonomics**: Simpler syntax, fewer steps
- **Discovery**: Clear error messages, helpful suggestions
- **Integration**: Works seamlessly with existing workflows