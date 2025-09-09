#!/usr/bin/env python3

"""
Load testing script for Whiteout performance analysis.
Generates various file patterns and measures processing time.
"""

import os
import time
import subprocess
import tempfile
import statistics
from pathlib import Path
from typing import List, Tuple
import json

class WhiteoutLoadTester:
    def __init__(self, whiteout_binary: str = "whiteout"):
        self.whiteout_binary = whiteout_binary
        self.results = []
        
    def generate_test_file(self, lines: int, decoration_rate: float = 0.1) -> str:
        """Generate a test file with specified number of lines and decoration rate."""
        content = []
        decorations_count = 0
        
        for i in range(lines):
            if i / lines < decoration_rate:
                # Add different types of decorations
                decoration_type = i % 3
                if decoration_type == 0:
                    content.append(f'let secret_{i} = "confidential_{i}"; // @whiteout: "REDACTED"')
                    decorations_count += 1
                elif decoration_type == 1:
                    content.append(f'// @whiteout-start')
                    content.append(f'const DEBUG_{i} = true;')
                    content.append(f'// @whiteout-end')
                    content.append(f'const DEBUG_{i} = false;')
                    decorations_count += 1
                else:
                    content.append(f'let url = "[[http://localhost:{i}||https://api.example.com]]"; // @whiteout-partial')
                    decorations_count += 1
            else:
                # Regular code line
                content.append(f'let variable_{i} = {i};')
        
        return '\n'.join(content), decorations_count
    
    def measure_operation(self, operation: str, content: str, iterations: int = 5) -> Tuple[float, float]:
        """Measure the time for a specific operation."""
        times = []
        
        with tempfile.NamedTemporaryFile(mode='w', suffix='.rs', delete=False) as f:
            f.write(content)
            temp_file = f.name
        
        try:
            for _ in range(iterations):
                start = time.perf_counter()
                result = subprocess.run(
                    [self.whiteout_binary, operation, temp_file],
                    capture_output=True,
                    text=True
                )
                end = time.perf_counter()
                
                if result.returncode == 0:
                    times.append((end - start) * 1000)  # Convert to milliseconds
        finally:
            os.unlink(temp_file)
        
        if times:
            return statistics.mean(times), statistics.stdev(times) if len(times) > 1 else 0
        return 0, 0
    
    def run_load_test(self):
        """Run comprehensive load tests."""
        print("=== Whiteout Load Testing ===\n")
        
        test_cases = [
            (100, 0.1, "Small file, low decoration"),
            (1000, 0.1, "Medium file, low decoration"),
            (1000, 0.5, "Medium file, high decoration"),
            (10000, 0.1, "Large file, low decoration"),
            (10000, 0.5, "Large file, high decoration"),
            (50000, 0.1, "Very large file, low decoration"),
        ]
        
        for lines, decoration_rate, description in test_cases:
            print(f"Testing: {description}")
            print(f"  Lines: {lines}, Decoration rate: {decoration_rate*100:.0f}%")
            
            content, decoration_count = self.generate_test_file(lines, decoration_rate)
            content_size_kb = len(content) / 1024
            
            print(f"  File size: {content_size_kb:.1f} KB")
            print(f"  Decorations: {decoration_count}")
            
            # Test clean operation
            clean_mean, clean_std = self.measure_operation("clean", content)
            print(f"  Clean: {clean_mean:.2f}ms ± {clean_std:.2f}ms")
            
            # Test smudge operation
            smudge_mean, smudge_std = self.measure_operation("smudge", content)
            print(f"  Smudge: {smudge_mean:.2f}ms ± {smudge_std:.2f}ms")
            
            # Calculate throughput
            if clean_mean > 0:
                clean_throughput = (content_size_kb / clean_mean) * 1000  # KB/s
                print(f"  Clean throughput: {clean_throughput:.1f} KB/s")
            
            if smudge_mean > 0:
                smudge_throughput = (content_size_kb / smudge_mean) * 1000  # KB/s
                print(f"  Smudge throughput: {smudge_throughput:.1f} KB/s")
            
            self.results.append({
                'description': description,
                'lines': lines,
                'decoration_rate': decoration_rate,
                'decorations': decoration_count,
                'file_size_kb': content_size_kb,
                'clean_ms': clean_mean,
                'clean_std': clean_std,
                'smudge_ms': smudge_mean,
                'smudge_std': smudge_std,
            })
            
            print()
    
    def run_stress_test(self):
        """Run stress test with many concurrent operations."""
        print("=== Stress Testing ===\n")
        
        import concurrent.futures
        import multiprocessing
        
        cpu_count = multiprocessing.cpu_count()
        print(f"Running stress test with {cpu_count} concurrent operations...\n")
        
        # Generate test content
        content, _ = self.generate_test_file(1000, 0.2)
        
        def process_file(index):
            with tempfile.NamedTemporaryFile(mode='w', suffix=f'_{index}.rs', delete=False) as f:
                f.write(content)
                temp_file = f.name
            
            try:
                start = time.perf_counter()
                subprocess.run(
                    [self.whiteout_binary, "clean", temp_file],
                    capture_output=True,
                    text=True
                )
                end = time.perf_counter()
                return (end - start) * 1000
            finally:
                os.unlink(temp_file)
        
        # Run concurrent operations
        with concurrent.futures.ThreadPoolExecutor(max_workers=cpu_count) as executor:
            start_time = time.perf_counter()
            futures = [executor.submit(process_file, i) for i in range(cpu_count * 10)]
            results = [f.result() for f in concurrent.futures.as_completed(futures)]
            total_time = (time.perf_counter() - start_time) * 1000
        
        print(f"Processed {len(results)} files in {total_time:.2f}ms")
        print(f"Average time per file: {statistics.mean(results):.2f}ms")
        print(f"Min/Max: {min(results):.2f}ms / {max(results):.2f}ms")
        print(f"Throughput: {len(results) / (total_time/1000):.1f} files/second")
    
    def generate_report(self):
        """Generate a performance report."""
        print("\n=== Performance Report ===\n")
        
        if not self.results:
            print("No test results available")
            return
        
        # Find bottlenecks
        slowest_clean = max(self.results, key=lambda x: x['clean_ms'])
        slowest_smudge = max(self.results, key=lambda x: x['smudge_ms'])
        
        print("Bottlenecks:")
        print(f"  Slowest clean: {slowest_clean['description']} ({slowest_clean['clean_ms']:.2f}ms)")
        print(f"  Slowest smudge: {slowest_smudge['description']} ({slowest_smudge['smudge_ms']:.2f}ms)")
        
        # Performance scaling
        print("\nPerformance Scaling:")
        for r in self.results:
            if r['lines'] > 0:
                ms_per_1k_lines_clean = (r['clean_ms'] / r['lines']) * 1000
                ms_per_1k_lines_smudge = (r['smudge_ms'] / r['lines']) * 1000
                print(f"  {r['description']}:")
                print(f"    Clean: {ms_per_1k_lines_clean:.2f}ms per 1K lines")
                print(f"    Smudge: {ms_per_1k_lines_smudge:.2f}ms per 1K lines")
        
        # Save results to JSON
        with open('load_test_results.json', 'w') as f:
            json.dump(self.results, f, indent=2)
        print("\nResults saved to load_test_results.json")

def main():
    # Check if whiteout binary exists
    whiteout_binary = "whiteout"
    
    # Try to find the binary
    possible_paths = [
        "./target/release/whiteout",
        "./target/debug/whiteout",
        "whiteout"
    ]
    
    for path in possible_paths:
        if os.path.exists(path) or subprocess.run(["which", path], capture_output=True).returncode == 0:
            whiteout_binary = path
            break
    
    print(f"Using whiteout binary: {whiteout_binary}\n")
    
    tester = WhiteoutLoadTester(whiteout_binary)
    
    try:
        tester.run_load_test()
        tester.run_stress_test()
        tester.generate_report()
    except Exception as e:
        print(f"Error during testing: {e}")
        return 1
    
    return 0

if __name__ == "__main__":
    exit(main())