# Contributing to Whiteout

First off, thank you for considering contributing to Whiteout! It's people like you that make Whiteout such a great tool.

## Code of Conduct

This project and everyone participating in it is governed by our Code of Conduct. By participating, you are expected to uphold this code. Please report unacceptable behavior to conduct@whiteout.dev.

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check existing issues as you might find out that you don't need to create one. When you are creating a bug report, please include as many details as possible:

* **Use a clear and descriptive title**
* **Describe the exact steps to reproduce the problem**
* **Provide specific examples to demonstrate the steps**
* **Describe the behavior you observed and expected**
* **Include screenshots if possible**
* **Include your environment details** (OS, Rust version, Git version)

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion, please include:

* **Use a clear and descriptive title**
* **Provide a step-by-step description of the suggested enhancement**
* **Provide specific examples to demonstrate the steps**
* **Describe the current behavior and expected behavior**
* **Explain why this enhancement would be useful**

### Pull Requests

1. Fork the repo and create your branch from `main`
2. If you've added code that should be tested, add tests
3. If you've changed APIs, update the documentation
4. Ensure the test suite passes (`cargo test`)
5. Make sure your code follows the style guidelines (`cargo fmt` and `cargo clippy`)
6. Issue that pull request!

## Development Process

### Setting Up Your Development Environment

```bash
# Fork and clone the repository
git clone https://github.com/yourusername/whiteout.git
cd whiteout

# Add upstream remote
git remote add upstream https://github.com/originalowner/whiteout.git

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build the project
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- init
```

### Project Structure

```
whiteout/
├── src/
│   ├── main.rs           # CLI entry point
│   ├── lib.rs            # Library exports
│   ├── parser/           # Decoration parsers
│   │   ├── inline.rs     # Inline decoration parser
│   │   ├── block.rs      # Block decoration parser
│   │   └── partial.rs    # Partial replacement parser
│   ├── transform/        # Git filter transformations
│   │   ├── clean.rs      # Clean filter (remove secrets)
│   │   └── smudge.rs     # Smudge filter (restore secrets)
│   ├── storage/          # Local value storage
│   │   ├── local.rs      # File-based storage
│   │   └── crypto.rs     # Encryption utilities
│   └── config/           # Configuration management
└── tests/                # Test suites
```

### Testing

We maintain high test coverage. Please add tests for any new functionality:

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_partial_parser

# Run integration tests only
cargo test --test integration_test

# Check test coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

### Code Style

We use standard Rust formatting and linting:

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run clippy for linting
cargo clippy -- -D warnings

# Fix clippy suggestions
cargo clippy --fix
```

### Commit Messages

We follow conventional commit messages:

* `feat:` New feature
* `fix:` Bug fix
* `docs:` Documentation changes
* `test:` Test additions or changes
* `refactor:` Code refactoring
* `style:` Code style changes
* `perf:` Performance improvements
* `chore:` Maintenance tasks

Example:
```
feat: add support for YAML configuration files

- Parse YAML decoration syntax
- Add tests for YAML files
- Update documentation
```

### Documentation

* Keep README.md up to date with any new features
* Add inline documentation for public APIs
* Update CHANGELOG.md following [Keep a Changelog](https://keepachangelog.com/)
* Generate and review docs with `cargo doc --open`

## Release Process

1. Update version in `Cargo.toml`
2. Update CHANGELOG.md
3. Create a pull request with version bump
4. After merge, tag the release: `git tag -a v0.2.0 -m "Release version 0.2.0"`
5. Push tags: `git push upstream --tags`
6. GitHub Actions will handle the rest

## Community

* Join our [Discord server](https://discord.gg/whiteout)
* Follow us on [Twitter](https://twitter.com/whiteoutdev)
* Read our [blog](https://blog.whiteout.dev)

## Recognition

Contributors will be recognized in:
* README.md contributors section
* Release notes
* Our website's contributors page

## Questions?

Feel free to open an issue with the `question` label or reach out on Discord!

Thank you for contributing to Whiteout! 🎉