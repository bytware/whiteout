#!/bin/bash

# Quick performance benchmark

echo "Creating test file..."
cat > test.rs << 'EOF'
fn main() {
    let api_key = "sk-12345"; // @whiteout: "REDACTED"
    let debug = true; // @whiteout: false
    // @whiteout-start
    const LOCAL_API = "http://localhost:3000";
    // @whiteout-end
    const LOCAL_API = "https://api.prod.com";
EOF

for i in {1..1000}; do
    if [ $((i % 50)) -eq 0 ]; then
        echo "    let key_$i = \"secret_$i\"; // @whiteout: \"REDACTED\"" >> test.rs
    else
        echo "    let var_$i = $i;" >> test.rs
    fi
done
echo "}" >> test.rs

echo "Test file: $(wc -l test.rs)"
echo ""

# Initialize whiteout
./target/release/whiteout init . 2>/dev/null || true

# Time the clean operation
echo "Testing clean filter..."
time ./target/release/whiteout clean test.rs > /dev/null 2>&1

echo ""
echo "Testing with stdin..."
time cat test.rs | ./target/release/whiteout clean > /dev/null 2>&1

# Memory usage
echo ""
echo "Memory usage check..."
/usr/bin/time -l ./target/release/whiteout clean test.rs 2>&1 | grep "maximum resident" || echo "Memory info not available"

rm -f test.rs