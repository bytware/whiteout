# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2025-01-11

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
- Core transformation engine with Rust implementation
- Git clean and smudge filter support
- Basic CLI with init, clean, smudge commands
- Support for inline decorations (`@whiteout:`)
- Support for block decorations (`@whiteout-start`/`@whiteout-end`)
- Support for partial replacements (`[[local||committed]]`)
- Local TOML-based storage system
- Configuration file support (`.whiteout/config.toml`)
- Automatic Git configuration during initialization

### Security
- Secrets never enter Git history
- Local storage with AES-256-GCM encryption option
- Automatic `.gitignore` creation for storage files
- Pre-commit hook support for additional validation
- Protection against accidental pattern matching
- Explicit decoration requirement for partial replacements

## Links
- [Compare v1.0.0...HEAD](https://github.com/bytware/whiteout/compare/v1.0.0...HEAD)
- [v1.0.0](https://github.com/bytware/whiteout/releases/tag/v1.0.0)