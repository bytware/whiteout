# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release of Whiteout
- Three decoration styles: inline, block, and partial replacements
- Git filter integration (clean/smudge)
- Local storage system with optional encryption
- CLI commands for initialization, marking, and status
- Language-agnostic support for any text file
- Comprehensive test suite with 39 tests
- Support for branch-specific configurations
- Safety feature: `@whiteout-partial` decorator required for partial replacements

### Security
- Secrets never enter Git history
- Local storage with AES-256-GCM encryption option
- Automatic `.gitignore` creation for storage files
- Pre-commit hook support for additional validation

## [0.1.0] - 2024-01-15

### Added
- Core transformation engine with Rust implementation
- Git clean and smudge filter support
- Basic CLI with init, clean, smudge commands
- Support for inline decorations (`@whiteout:`)
- Support for block decorations (`@whiteout-start`/`@whiteout-end`)
- Support for partial replacements (`[[local||committed]]`)
- Local TOML-based storage system
- Configuration file support (`.whiteout/config.toml`)
- Automatic Git configuration during initialization

### Changed
- Partial replacements now require explicit `@whiteout-partial` decorator for safety

### Fixed
- Block decoration preservation in commits
- Line number shifting in smudge filter
- Concurrent access in tests
- Partial replacement pattern restoration

### Security
- Added protection against accidental pattern matching
- Implemented explicit decoration requirement for partial replacements

## Links
- [Compare v0.1.0...HEAD](https://github.com/yourusername/whiteout/compare/v0.1.0...HEAD)
- [v0.1.0](https://github.com/yourusername/whiteout/releases/tag/v0.1.0)