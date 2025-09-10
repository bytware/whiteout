#!/bin/bash

# Comprehensive End-to-End Git Integration Test for Whiteout
# This test verifies that secrets never reach Git history while remaining in working directory

set -e  # Exit on first error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get script directory and project root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

echo -e "${BLUE}=== Whiteout End-to-End Git Integration Test ===${NC}"
echo

# Step 1: Build whiteout
echo -e "${YELLOW}Step 1: Building whiteout...${NC}"
cd "$PROJECT_ROOT"
if command -v cargo &> /dev/null; then
    cargo build --release 2>/dev/null || cargo build
    WHITEOUT_BIN="$PROJECT_ROOT/target/release/whiteout"
    if [ ! -f "$WHITEOUT_BIN" ]; then
        WHITEOUT_BIN="$PROJECT_ROOT/target/debug/whiteout"
    fi
else
    echo -e "${RED}Error: Cargo not found. Please install Rust.${NC}"
    exit 1
fi

if [ ! -f "$WHITEOUT_BIN" ]; then
    echo -e "${RED}Error: whiteout binary not found at $WHITEOUT_BIN${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Whiteout built successfully${NC}"
echo

# Step 2: Create test repository
echo -e "${YELLOW}Step 2: Creating test repository...${NC}"
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"
echo "Test directory: $TEST_DIR"

# Initialize git repo
git init -q
git config user.email "test@example.com"
git config user.name "Test User"
echo -e "${GREEN}✓ Git repository initialized${NC}"
echo

# Step 3: Initialize whiteout
echo -e "${YELLOW}Step 3: Initializing whiteout...${NC}"
"$WHITEOUT_BIN" init .

# Configure Git filters with absolute path
git config filter.whiteout.clean "$WHITEOUT_BIN clean"
git config filter.whiteout.smudge "$WHITEOUT_BIN smudge"
git config filter.whiteout.required true

# Verify .gitattributes was created
if [ ! -f .gitattributes ] || ! grep -q "filter=whiteout" .gitattributes; then
    echo -e "${RED}Error: .gitattributes not properly configured${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Whiteout initialized and Git filters configured${NC}"
echo

# Step 4: Create comprehensive test file with all decoration types
echo -e "${YELLOW}Step 4: Creating test file with all decoration types...${NC}"

cat > test_secrets.js << 'EOF'
// Test file with various secret decorations

// 1. Inline decoration - API keys and tokens
const API_KEY = "sk-proj-SUPER-SECRET-KEY-123456789"; // @whiteout: process.env.API_KEY
const AUTH_TOKEN = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"; // @whiteout: process.env.AUTH_TOKEN
const PASSWORD = "admin123!@#"; // @whiteout: "REDACTED"

// 2. Block decoration - Debug configuration
// @whiteout-start
const DEBUG_CONFIG = {
    enabled: true,
    verbose: true,
    logLevel: "trace",
    dbUrl: "postgresql://admin:secretpass@localhost:5432/devdb"
};
// @whiteout-end
const DEBUG_CONFIG = {
    enabled: false,
    verbose: false,
    logLevel: "error",
    dbUrl: process.env.DATABASE_URL
};

// 3. Partial decoration - URLs with embedded credentials
const API_URL = "https://[[admin:pass123@dev.localhost:8080||api.production.com]]/v1/endpoint"; // @whiteout-partial
const WEBHOOK = "https://[[secret-token@internal.dev||webhook.example.com]]/notify"; // @whiteout-partial

// 4. Mixed decorations in JSON config
const config = {
    apiKey: "sk-live-xyz789", // @whiteout: process.env.LIVE_API_KEY
    endpoint: "https://[[staging.internal||api.example.com]]/graphql", // @whiteout-partial
    // @whiteout-start
    credentials: {
        username: "developer",
        password: "dev123456"
    },
    // @whiteout-end
    credentials: null,
};

// 5. Test that non-decorated patterns are NOT transformed
const normalArray = matrix[[row||col]];  // This should NOT be transformed (no decorator)
const regexPattern = "[[a-z]||[0-9]]";   // This should NOT be transformed (no decorator)
EOF

echo "Original file content:"
cat test_secrets.js
echo
echo -e "${GREEN}✓ Test file created with all decoration types${NC}"
echo

# Step 5: Stage and commit the file
echo -e "${YELLOW}Step 5: Staging and committing file...${NC}"
git add test_secrets.js
git commit -m "Add test file with secrets" -q
echo -e "${GREEN}✓ File committed to Git${NC}"
echo

# Step 6: Verify committed content (the critical test!)
echo -e "${YELLOW}Step 6: Verifying committed content in Git...${NC}"
COMMITTED_CONTENT=$(git show HEAD:test_secrets.js)

# Helper function to check for secret in committed content
check_not_in_commit() {
    local secret="$1"
    local description="$2"
    if echo "$COMMITTED_CONTENT" | grep -qF "$secret"; then
        echo -e "${RED}❌ CRITICAL ERROR: $description found in Git commit!${NC}"
        echo -e "${RED}   Secret: '$secret'${NC}"
        return 1
    else
        echo -e "${GREEN}✓ $description NOT in commit${NC}"
        return 0
    fi
}

# Helper function to check for safe value in committed content
check_in_commit() {
    local value="$1"
    local description="$2"
    if echo "$COMMITTED_CONTENT" | grep -qF "$value"; then
        echo -e "${GREEN}✓ $description present in commit${NC}"
        return 0
    else
        echo -e "${RED}❌ ERROR: $description missing from commit!${NC}"
        echo -e "${RED}   Expected: '$value'${NC}"
        return 1
    fi
}

# Check that secrets are NOT in the commit
ERRORS=0
check_not_in_commit "sk-proj-SUPER-SECRET-KEY-123456789" "API key" || ((ERRORS++))
check_not_in_commit "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9" "Auth token" || ((ERRORS++))
check_not_in_commit "admin123!@#" "Password" || ((ERRORS++))
check_not_in_commit "enabled: true" "Debug enabled flag" || ((ERRORS++))
check_not_in_commit "logLevel: \"trace\"" "Debug log level" || ((ERRORS++))
check_not_in_commit "postgresql://admin:secretpass@localhost" "Database URL with password" || ((ERRORS++))
check_not_in_commit "admin:pass123@dev.localhost:8080" "URL with embedded credentials" || ((ERRORS++))
check_not_in_commit "secret-token@internal.dev" "Webhook token" || ((ERRORS++))
check_not_in_commit "sk-live-xyz789" "Live API key" || ((ERRORS++))
check_not_in_commit "staging.internal" "Internal staging URL" || ((ERRORS++))
check_not_in_commit "username: \"developer\"" "Developer username" || ((ERRORS++))
check_not_in_commit "password: \"dev123456\"" "Developer password" || ((ERRORS++))

echo

# Check that safe replacements ARE in the commit
check_in_commit "process.env.API_KEY" "API key replacement" || ((ERRORS++))
check_in_commit "process.env.AUTH_TOKEN" "Auth token replacement" || ((ERRORS++))
check_in_commit "REDACTED" "Password redaction" || ((ERRORS++))
check_in_commit "enabled: false" "Production debug flag" || ((ERRORS++))
check_in_commit "logLevel: \"error\"" "Production log level" || ((ERRORS++))
check_in_commit "process.env.DATABASE_URL" "Database URL replacement" || ((ERRORS++))
check_in_commit "api.production.com" "Production API URL" || ((ERRORS++))
check_in_commit "webhook.example.com" "Production webhook URL" || ((ERRORS++))
check_in_commit "credentials: null" "Null credentials" || ((ERRORS++))

echo

# Check that non-decorated patterns were NOT transformed
check_in_commit "matrix[[row||col]]" "Non-decorated array pattern" || ((ERRORS++))
check_in_commit "[[a-z]||[0-9]]" "Non-decorated regex pattern" || ((ERRORS++))

echo

if [ $ERRORS -gt 0 ]; then
    echo -e "${RED}❌ CRITICAL: $ERRORS security check(s) failed!${NC}"
    echo -e "${RED}Secrets may have been committed to Git!${NC}"
    echo
    echo "Full committed content:"
    echo "$COMMITTED_CONTENT"
    exit 1
fi

echo -e "${GREEN}✅ All secrets successfully filtered from Git commit${NC}"
echo

# Step 7: Verify working directory still has secrets
echo -e "${YELLOW}Step 7: Verifying working directory has secrets...${NC}"

check_in_working() {
    local secret="$1"
    local description="$2"
    if grep -qF "$secret" test_secrets.js; then
        echo -e "${GREEN}✓ $description present in working directory${NC}"
        return 0
    else
        echo -e "${RED}❌ ERROR: $description missing from working directory!${NC}"
        return 1
    fi
}

WD_ERRORS=0
check_in_working "sk-proj-SUPER-SECRET-KEY-123456789" "API key" || ((WD_ERRORS++))
check_in_working "admin123!@#" "Password" || ((WD_ERRORS++))
check_in_working "enabled: true" "Debug flag" || ((WD_ERRORS++))
check_in_working "admin:pass123@dev.localhost:8080" "URL credentials" || ((WD_ERRORS++))

if [ $WD_ERRORS -gt 0 ]; then
    echo -e "${RED}❌ ERROR: $WD_ERRORS secret(s) missing from working directory!${NC}"
    exit 1
fi

echo

# Step 8: Test checkout restoration (smudge filter)
echo -e "${YELLOW}Step 8: Testing checkout restoration...${NC}"
rm test_secrets.js
git checkout test_secrets.js 2>/dev/null

if check_in_working "sk-proj-SUPER-SECRET-KEY-123456789" "API key after checkout"; then
    echo -e "${GREEN}✅ Smudge filter successfully restored secrets${NC}"
else
    echo -e "${RED}❌ ERROR: Smudge filter failed to restore secrets${NC}"
    exit 1
fi

echo

# Step 9: Test with a new branch
echo -e "${YELLOW}Step 9: Testing branch switching...${NC}"
git checkout -b test-branch -q
echo "// New comment" >> test_secrets.js
git add test_secrets.js
git commit -m "Test on branch" -q
git checkout main -q

if check_in_working "sk-proj-SUPER-SECRET-KEY-123456789" "API key after branch switch"; then
    echo -e "${GREEN}✅ Secrets preserved across branch switches${NC}"
else
    echo -e "${RED}❌ ERROR: Secrets lost during branch switch${NC}"
    exit 1
fi

echo

# Final summary
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✅ ALL END-TO-END TESTS PASSED SUCCESSFULLY!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo
echo "Summary:"
echo "• Secrets are kept in working directory"
echo "• Secrets are filtered from Git commits"
echo "• Safe replacement values are committed"
echo "• Secrets are restored on checkout"
echo "• Non-decorated patterns are untouched"
echo "• All decoration types work correctly"
echo

# Cleanup
echo "Cleaning up test directory..."
cd /
rm -rf "$TEST_DIR"

echo -e "${GREEN}Test completed successfully!${NC}"