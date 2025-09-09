# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Whiteout is a Rust-based Git filter tool that prevents secrets and local-only code from being committed while maintaining them in the working directory. It uses decoration syntax to mark code sections as local-only.

## Development Commands

### Build & Run
```bash
# Build the project
cargo build

# Build optimized release version
cargo build --release

# Run the CLI tool
cargo run -- <command>

# Install to system (requires sudo)
./install.sh
```

### Testing
```bash
# Run all tests
cargo test

# Run specific test
cargo test <test_name>

# Run integration tests
cargo test --test integration_test

# Run Git integration test script
bash tests/git_integration_test.sh

# Run with verbose output
cargo test -- --nocapture
```

### Development Tools
```bash
# Format code
cargo fmt

# Check for linting issues
cargo clippy

# Generate documentation
cargo doc --open

# Check for compilation errors without building
cargo check
```

## Architecture

### Core Components

1. **Parser Module** (`src/parser/`)
   - `inline.rs`: Handles inline decorations (`// @whiteout: replacement`)
   - `block.rs`: Handles block decorations (`@whiteout-start`/`@whiteout-end`)
   - `partial.rs`: Handles partial replacements (`[[local||committed]]`)

2. **Transform Module** (`src/transform/`)
   - `clean.rs`: Git clean filter - strips local values before commit
   - `smudge.rs`: Git smudge filter - restores local values after checkout

3. **Storage Module** (`src/storage/`)
   - `local.rs`: Manages `.whiteout/local.toml` for storing local values
   - `crypto.rs`: Optional encryption for sensitive local values

4. **Config Module** (`src/config/`)
   - Manages project-level whiteout configuration

### Git Filter Integration

The tool integrates with Git through clean/smudge filters:
- **Clean**: `Working Directory → Repository` (removes secrets)
- **Smudge**: `Repository → Working Directory` (restores secrets)

Configuration requires:
```bash
git config filter.whiteout.clean 'whiteout clean'
git config filter.whiteout.smudge 'whiteout smudge'
git config filter.whiteout.required true
```

And `.gitattributes`:
```
* filter=whiteout
```

### Decoration Patterns

1. **Inline**: `value // @whiteout: replacement`
2. **Block**: Between `@whiteout-start` and `@whiteout-end` comments
3. **Partial**: `[[local_value||committed_value]]` within strings

## Key Implementation Details

- Local values stored in `.whiteout/local.toml` (gitignored)
- Branch-specific storage supported via Git branch detection
- Language-agnostic parsing based on comment patterns
- Preserves exact formatting and indentation during transformation
- Atomic file operations to prevent data loss

## Testing Approach

- Unit tests for each parser type
- Integration tests for complete transformation pipeline
- Shell script for real Git workflow testing
- Test fixtures in `tests/` directory