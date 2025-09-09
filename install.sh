#!/bin/bash

set -e

echo "Installing Whiteout - Local-Only Code Decoration Tool"
echo "====================================================="
echo

# Check for cargo in common locations
if command -v cargo &> /dev/null; then
    CARGO="cargo"
elif [ -x "$HOME/.cargo/bin/cargo" ]; then
    CARGO="$HOME/.cargo/bin/cargo"
else
    echo "Error: Rust/Cargo is not installed."
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

echo "Building Whiteout..."
$CARGO build --release

echo
echo "Installing binary..."
sudo cp target/release/whiteout /usr/local/bin/

echo
echo "Whiteout has been installed successfully!"
echo
echo "To initialize Whiteout in your project, run:"
echo "  whiteout init"
echo
echo "Then configure Git filters:"
echo "  git config filter.whiteout.clean 'whiteout clean'"
echo "  git config filter.whiteout.smudge 'whiteout smudge'"
echo "  git config filter.whiteout.required true"
echo
echo "For more information, run:"
echo "  whiteout --help"