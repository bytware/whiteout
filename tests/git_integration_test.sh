#!/bin/bash
set -e

echo "=== Whiteout Git Integration Test ==="
echo

# Build whiteout
echo "Building whiteout..."
cargo build --release 2>/dev/null || cargo build

# Create test directory
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"
echo "Test directory: $TEST_DIR"

# Initialize git repo
git init
git config user.email "test@example.com"
git config user.name "Test User"

# Initialize whiteout
echo "Initializing whiteout..."
/root/repo/target/debug/whiteout init

# Override Git config to use full path
git config filter.whiteout.clean '/root/repo/target/debug/whiteout clean'
git config filter.whiteout.smudge '/root/repo/target/debug/whiteout smudge'

# Create a file with secrets
cat > config.js << 'EOF'
const API_KEY = "sk-proj-ACTUAL-SECRET-KEY-123456789"; // @whiteout: process.env.API_KEY
const DB_PASSWORD = "super-secret-password"; // @whiteout: "REDACTED"
const DEBUG = true; // @whiteout: false
const URL = "https://[[admin:pass@localhost:3000||api.example.com]]/v1";
EOF

echo "Original file content:"
cat config.js
echo

# Stage the file (this should trigger the clean filter)
echo "Staging file (running clean filter)..."
git add config.js

# Check what would be committed
echo "Content that will be committed (from git diff --cached):"
git diff --cached config.js | grep '^+' | grep -v '^+++' || true
echo

# Commit
git commit -m "Add config with secrets"

# Check what was actually committed
echo "Content actually committed to Git:"
git show HEAD:config.js
echo

# Verify secrets are NOT in the commit
echo "Checking for secrets in committed content..."
if git show HEAD:config.js | grep -q "sk-proj-ACTUAL-SECRET-KEY-123456789"; then
    echo "❌ CRITICAL ERROR: Secret API key found in commit!"
    exit 1
else
    echo "✓ API key not in commit"
fi

if git show HEAD:config.js | grep -q "super-secret-password"; then
    echo "❌ CRITICAL ERROR: Password found in commit!"
    exit 1
else
    echo "✓ Password not in commit"
fi

if git show HEAD:config.js | grep -q "admin:pass@localhost"; then
    echo "❌ CRITICAL ERROR: URL credentials found in commit!"
    exit 1
else
    echo "✓ URL credentials not in commit"
fi

# Check working directory still has secrets
echo
echo "Checking working directory still has secrets..."
if grep -q "sk-proj-ACTUAL-SECRET-KEY-123456789" config.js; then
    echo "✓ Secret still in working directory"
else
    echo "❌ ERROR: Secret missing from working directory"
    exit 1
fi

# Test checkout (smudge filter)
echo
echo "Testing checkout with smudge filter..."
rm config.js
git checkout config.js

echo "Content after checkout:"
cat config.js
echo

# Verify secrets are restored
if grep -q "sk-proj-ACTUAL-SECRET-KEY-123456789" config.js; then
    echo "✓ Secret restored after checkout"
else
    echo "❌ ERROR: Secret not restored after checkout"
    exit 1
fi

echo
echo "=== ✅ All Git integration tests passed! ==="
echo "Secrets are kept local and never committed to Git."

# Cleanup
cd /
rm -rf "$TEST_DIR"