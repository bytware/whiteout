# Whiteout - Local-Only Code Decoration Tool

Whiteout is a Git filter tool that allows you to keep local-only code (like API keys, debug settings, or development URLs) in your working directory while ensuring they never get committed to your repository.

## Features

- 🔒 **Secure**: Prevents secrets from being committed to Git
- 🎯 **Flexible**: Multiple decoration styles (inline, block, partial)
- 🚀 **Fast**: Efficient Rust implementation
- 🔧 **Easy Integration**: Works seamlessly with Git filters

## Installation

```bash
# Clone and build
git clone https://github.com/terragon-labs/whiteout
cd whiteout
cargo build --release

# Install to system
sudo cp target/release/whiteout /usr/local/bin/
```

## Quick Start

1. Initialize Whiteout in your project:
```bash
whiteout init
```

2. Configure Git filters:
```bash
git config filter.whiteout.clean 'whiteout clean'
git config filter.whiteout.smudge 'whiteout smudge'
git config filter.whiteout.required true
```

3. Add to `.gitattributes`:
```
* filter=whiteout
```

## Decoration Syntax

### Inline Decoration
Keep a local value while committing a safe alternative:
```javascript
let apiKey = "sk-my-secret-key"; // @whiteout: process.env.API_KEY
```

### Block Decoration
Maintain entire blocks of local-only code:
```rust
// @whiteout-start
const DEBUG: bool = true;
const LOG_LEVEL: &str = "trace";
// @whiteout-end
const DEBUG: bool = false;
const LOG_LEVEL: &str = "error";
```

### Partial Decoration
Replace parts of strings:
```python
url = "https://[[localhost:8080||api.production.com]]/v1"
```

## How It Works

Whiteout uses Git's clean/smudge filter system:

1. **Working Directory** (your local code with secrets)
   - Contains decorated code with local values
   
2. **Clean Filter** (when staging/committing)
   - Stores local values securely in `.whiteout/local.toml`
   - Replaces local values with safe committed values
   - Preserves decoration markers
   
3. **Repository** (what gets committed)
   - Contains only safe, committed values
   - No secrets or local configurations
   
4. **Smudge Filter** (when checking out)
   - Restores local values from storage
   - Maintains your local development environment

## Commands

- `whiteout init` - Initialize in current project
- `whiteout clean` - Apply clean filter (used by Git)
- `whiteout smudge` - Apply smudge filter (used by Git)
- `whiteout mark <file>` - Mark code as local-only
- `whiteout unmark <file>` - Remove decorations
- `whiteout status` - Show decorated files
- `whiteout config` - Manage settings

## Example Workflow

```bash
# 1. Write code with local values
echo 'let key = "sk-12345"; // @whiteout: env("KEY")' > config.js

# 2. Stage the file (Git runs clean filter automatically)
git add config.js

# 3. Commit (only safe value is stored)
git commit -m "Add config"

# 4. The committed file contains: 
# let key = env("KEY"); // @whiteout: env("KEY")

# 5. In your working directory, you still see:
# let key = "sk-12345"; // @whiteout: env("KEY")
```

## Security

- Local values stored in `.whiteout/local.toml` (gitignored)
- Optional encryption for sensitive data
- Values are branch-specific
- Never commits actual secrets to Git history

## Development

```bash
# Run tests
cargo test

# Run example
cargo run --example demo

# Build documentation
cargo doc --open
```

## License

MIT License - See LICENSE file for details

## Contributing

Contributions are welcome! Please read CONTRIBUTING.md for guidelines.

## Support

For issues and questions, please use the GitHub issue tracker.