#!/bin/bash

# Second-Pass Performance Audit Script for Whiteout
# Measures real-world performance after initial optimizations

set -e

echo "========================================="
echo "WHITEOUT PERFORMANCE AUDIT - SECOND PASS"
echo "========================================="
echo ""

# Build release version with various optimization levels
echo "Building release versions..."
echo "----------------------------"

# Standard release build
~/.cargo/bin/cargo build --release 2>&1 | tail -3

# Check binary size
echo ""
echo "Binary Analysis:"
echo "----------------"
ls -lh target/release/whiteout
file target/release/whiteout
echo ""

# Create test data
echo "Creating test data..."
echo "--------------------"
mkdir -p test_data

# Small file (100 lines)
cat > test_data/small.rs << 'EOF'
fn main() {
    let api_key = "sk-12345"; // @whiteout: "REDACTED"
    let debug = true; // @whiteout: false
    
    // @whiteout-start
    const LOCAL_API = "http://localhost:3000";
    const LOCAL_DEBUG = true;
    // @whiteout-end
    const LOCAL_API = "https://api.prod.com";
    const LOCAL_DEBUG = false;
    
    for i in 0..100 {
        println!("Line {}", i);
EOF

for i in {1..87}; do
    echo "        let var_$i = $i;" >> test_data/small.rs
done
echo "    }" >> test_data/small.rs
echo "}" >> test_data/small.rs

# Medium file (1000 lines)
echo "fn main() {" > test_data/medium.rs
for i in {1..980}; do
    if [ $((i % 50)) -eq 0 ]; then
        echo "    let key_$i = \"secret_$i\"; // @whiteout: \"REDACTED\"" >> test_data/medium.rs
    elif [ $((i % 100)) -eq 0 ]; then
        echo "    // @whiteout-start" >> test_data/medium.rs
        echo "    const DEBUG_$i = true;" >> test_data/medium.rs
        echo "    // @whiteout-end" >> test_data/medium.rs
        echo "    const DEBUG_$i = false;" >> test_data/medium.rs
    else
        echo "    let var_$i = $i;" >> test_data/medium.rs
    fi
done
echo "}" >> test_data/medium.rs

# Large file (10000 lines) 
echo "fn main() {" > test_data/large.rs
for i in {1..9980}; do
    if [ $((i % 100)) -eq 0 ]; then
        echo "    let key_$i = \"secret_$i\"; // @whiteout: \"REDACTED\"" >> test_data/large.rs
    elif [ $((i % 200)) -eq 0 ]; then
        echo "    // @whiteout-start" >> test_data/large.rs
        echo "    const DEBUG_$i = true;" >> test_data/large.rs
        echo "    // @whiteout-end" >> test_data/large.rs
        echo "    const DEBUG_$i = false;" >> test_data/large.rs
    else
        echo "    let var_$i = $i;" >> test_data/large.rs
    fi
done
echo "}" >> test_data/large.rs

echo "Test files created:"
wc -l test_data/*.rs
echo ""

# Function to measure time with high precision
measure_time() {
    local cmd="$1"
    local label="$2"
    
    # Use gtime if available (macOS with GNU coreutils), otherwise use time
    if command -v gtime &> /dev/null; then
        TIME_CMD="gtime"
    else
        TIME_CMD="time"
    fi
    
    # Run 5 times and get average
    echo -n "$label: "
    total=0
    for i in {1..5}; do
        # Measure in milliseconds
        result=$( { $TIME_CMD -f "%e" $cmd > /dev/null; } 2>&1 )
        total=$(echo "$total + $result" | bc)
    done
    avg=$(echo "scale=3; $total / 5" | bc)
    echo "${avg}s (avg of 5 runs)"
}

echo "Performance Benchmarks:"
echo "----------------------"

# Test clean filter performance
echo ""
echo "Clean Filter Performance:"
measure_time "cat test_data/small.rs | target/release/whiteout clean" "Small file (100 lines)"
measure_time "cat test_data/medium.rs | target/release/whiteout clean" "Medium file (1K lines)"
measure_time "cat test_data/large.rs | target/release/whiteout clean" "Large file (10K lines)"

# Test smudge filter performance  
echo ""
echo "Smudge Filter Performance:"
measure_time "cat test_data/small.rs | target/release/whiteout smudge" "Small file (100 lines)"
measure_time "cat test_data/medium.rs | target/release/whiteout smudge" "Medium file (1K lines)"
measure_time "cat test_data/large.rs | target/release/whiteout smudge" "Large file (10K lines)"

# Memory usage analysis
echo ""
echo "Memory Usage Analysis:"
echo "---------------------"
if command -v /usr/bin/time &> /dev/null; then
    echo "Small file:"
    /usr/bin/time -l target/release/whiteout clean test_data/small.rs 2>&1 | grep -E "maximum resident|elapsed" | head -2
    
    echo "Large file:"
    /usr/bin/time -l target/release/whiteout clean test_data/large.rs 2>&1 | grep -E "maximum resident|elapsed" | head -2
fi

# Run Criterion benchmarks
echo ""
echo "Running Criterion Benchmarks..."
echo "-------------------------------"
~/.cargo/bin/cargo bench --bench parser_benchmark -- --quick 2>&1 | grep -E "time:|found|parser_" | head -20

# Profile with Instruments if available (macOS)
if command -v instruments &> /dev/null; then
    echo ""
    echo "Profiling with Instruments (if authorized)..."
    echo "--------------------------------------------"
    # This will fail without proper authorization, but we try
    instruments -t "Time Profiler" -D profile.trace target/release/whiteout clean test_data/large.rs 2>/dev/null || echo "Instruments profiling requires authorization"
fi

# Check for performance regression opportunities
echo ""
echo "Performance Analysis Summary:"
echo "----------------------------"
echo "1. Binary size: $(ls -lh target/release/whiteout | awk '{print $5}')"
echo "2. Test file sizes: Small=$(wc -l < test_data/small.rs)L, Medium=$(wc -l < test_data/medium.rs)L, Large=$(wc -l < test_data/large.rs)L"
echo ""

# Cleanup
rm -rf test_data

echo ""
echo "Audit complete!"