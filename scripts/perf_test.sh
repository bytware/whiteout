#!/bin/bash

# Performance testing script for Whiteout
# Tests the impact of whiteout filters on Git operations

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TEST_DIR="/tmp/whiteout_perf_test"

echo "=== Whiteout Performance Test Suite ==="
echo "Testing performance impact of Git filters..."
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Cleanup function
cleanup() {
    if [ -d "$TEST_DIR" ]; then
        rm -rf "$TEST_DIR"
    fi
}

# Setup test environment
setup_test_env() {
    echo "Setting up test environment..."
    cleanup
    mkdir -p "$TEST_DIR"
    cd "$TEST_DIR"
    git init
    
    # Copy whiteout binary if it exists
    if [ -f "$PROJECT_ROOT/target/release/whiteout" ]; then
        cp "$PROJECT_ROOT/target/release/whiteout" /tmp/whiteout_test
        export PATH="/tmp:$PATH"
    else
        echo -e "${YELLOW}Warning: Release binary not found. Building...${NC}"
        cd "$PROJECT_ROOT"
        cargo build --release
        cd "$TEST_DIR"
        cp "$PROJECT_ROOT/target/release/whiteout" /tmp/whiteout_test
        export PATH="/tmp:$PATH"
    fi
}

# Generate test files with various decoration patterns
generate_test_files() {
    local num_files=$1
    local lines_per_file=$2
    local decorations_percent=$3
    
    echo "Generating $num_files test files with $lines_per_file lines each..."
    echo "Decoration density: $decorations_percent%"
    
    for ((i=1; i<=num_files; i++)); do
        {
            echo "// Test file $i"
            for ((j=1; j<=lines_per_file; j++)); do
                if [ $((RANDOM % 100)) -lt $decorations_percent ]; then
                    # Add decoration
                    case $((RANDOM % 3)) in
                        0)
                            echo "let secret_$j = \"confidential_$j\"; // @whiteout: \"REDACTED\""
                            ;;
                        1)
                            echo "// @whiteout-start"
                            echo "const DEBUG_$j = true;"
                            echo "// @whiteout-end"
                            echo "const DEBUG_$j = false;"
                            ;;
                        2)
                            echo "let url = \"[[http://localhost:$j||https://api.example.com]]\"; // @whiteout-partial"
                            ;;
                    esac
                else
                    echo "let variable_$j = $j;"
                fi
            done
        } > "test_file_$i.rs"
    done
}

# Measure time for a command
measure_time() {
    local cmd="$1"
    local start=$(date +%s%N)
    eval "$cmd" > /dev/null 2>&1
    local end=$(date +%s%N)
    echo $(( (end - start) / 1000000 )) # Return milliseconds
}

# Test Git operations without whiteout
test_without_whiteout() {
    echo -e "\n${GREEN}Testing WITHOUT Whiteout filters...${NC}"
    
    local add_time=$(measure_time "git add .")
    local commit_time=$(measure_time "git commit -m 'Test commit'")
    local checkout_time=$(measure_time "git checkout -b test-branch")
    
    echo "  git add:      ${add_time}ms"
    echo "  git commit:   ${commit_time}ms"
    echo "  git checkout: ${checkout_time}ms"
    
    echo $(( add_time + commit_time + checkout_time ))
}

# Test Git operations with whiteout
test_with_whiteout() {
    echo -e "\n${GREEN}Testing WITH Whiteout filters...${NC}"
    
    # Initialize whiteout
    whiteout_test init . > /dev/null 2>&1
    
    local add_time=$(measure_time "git add .")
    local commit_time=$(measure_time "git commit -m 'Test commit with whiteout'")
    local checkout_time=$(measure_time "git checkout -b test-branch-whiteout")
    
    echo "  git add:      ${add_time}ms"
    echo "  git commit:   ${commit_time}ms"
    echo "  git checkout: ${checkout_time}ms"
    
    echo $(( add_time + commit_time + checkout_time ))
}

# Test single file processing
test_single_file_performance() {
    echo -e "\n${GREEN}Testing single file processing...${NC}"
    
    # Generate a large test file
    {
        for ((i=1; i<=10000; i++)); do
            if [ $((i % 100)) -eq 0 ]; then
                echo "let key_$i = \"secret_$i\"; // @whiteout: \"REDACTED\""
            else
                echo "let var_$i = $i;"
            fi
        done
    } > large_test.rs
    
    # Test clean filter
    local clean_time=$(measure_time "whiteout_test clean large_test.rs")
    echo "  Clean filter (10k lines): ${clean_time}ms"
    
    # Test smudge filter
    local smudge_time=$(measure_time "whiteout_test smudge large_test.rs")
    echo "  Smudge filter (10k lines): ${smudge_time}ms"
}

# Memory usage test
test_memory_usage() {
    echo -e "\n${GREEN}Testing memory usage...${NC}"
    
    # Generate very large file
    {
        for ((i=1; i<=100000; i++)); do
            echo "let variable_$i = \"some_value_$i\";"
        done
    } > huge_test.rs
    
    if command -v /usr/bin/time &> /dev/null; then
        echo "  Processing 100k line file:"
        /usr/bin/time -v whiteout_test clean huge_test.rs 2>&1 | grep "Maximum resident" | sed 's/^/    /'
    else
        echo "  Memory profiling requires GNU time"
    fi
}

# Main test execution
main() {
    echo "Starting performance tests..."
    trap cleanup EXIT
    
    setup_test_env
    
    # Test different file counts and sizes
    for test_case in "10 1000 10" "100 100 10" "1000 10 5"; do
        set -- $test_case
        num_files=$1
        lines=$2
        decoration_percent=$3
        
        echo -e "\n${YELLOW}=== Test Case: $num_files files, $lines lines, $decoration_percent% decorations ===${NC}"
        
        # Clean state
        rm -rf .git *.rs
        git init > /dev/null 2>&1
        
        generate_test_files $num_files $lines $decoration_percent
        
        # Test without whiteout
        time_without=$(test_without_whiteout)
        
        # Reset for whiteout test
        rm -rf .git .whiteout
        git init > /dev/null 2>&1
        
        # Test with whiteout
        time_with=$(test_with_whiteout)
        
        # Calculate overhead
        overhead=$(( time_with - time_without ))
        overhead_percent=$(( (overhead * 100) / time_without ))
        
        echo -e "\n  ${YELLOW}Performance Impact:${NC}"
        echo "    Without Whiteout: ${time_without}ms"
        echo "    With Whiteout:    ${time_with}ms"
        echo "    Overhead:         ${overhead}ms (${overhead_percent}%)"
        
        if [ $overhead_percent -gt 50 ]; then
            echo -e "    ${RED}⚠ High overhead detected!${NC}"
        elif [ $overhead_percent -gt 20 ]; then
            echo -e "    ${YELLOW}⚠ Moderate overhead${NC}"
        else
            echo -e "    ${GREEN}✓ Acceptable overhead${NC}"
        fi
    done
    
    # Additional tests
    test_single_file_performance
    test_memory_usage
    
    echo -e "\n${GREEN}=== Performance tests completed ===${NC}"
}

# Run if executed directly
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    main "$@"
fi