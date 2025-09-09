use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use whiteout::parser::{Parser, Decoration, optimized};

fn generate_test_content_with_decorations(lines: usize) -> (String, Vec<Decoration>) {
    let mut content = Vec::new();
    let mut decorations = Vec::new();
    
    for i in 0..lines {
        if i % 50 == 0 {
            // Add inline decoration
            content.push(format!(r#"let api_key_{} = "sk-12345-{}"; // @whiteout: "REDACTED""#, i, i));
            decorations.push(Decoration::Inline {
                line: i + 1,
                local_value: format!(r#"let api_key_{} = "sk-12345-{}";"#, i, i),
                committed_value: "\"REDACTED\"".to_string(),
            });
        } else if i % 100 == 0 {
            // Add block decoration
            content.push("// @whiteout-start".to_string());
            content.push(format!("const DEBUG_{} = true;", i));
            content.push("// @whiteout-end".to_string());
            content.push(format!("const DEBUG_{} = false;", i));
            
            decorations.push(Decoration::Block {
                start_line: i + 1,
                end_line: i + 3,
                local_content: format!("const DEBUG_{} = true;", i),
                committed_content: format!("const DEBUG_{} = false;", i),
            });
        } else {
            content.push(format!("let var_{} = {};", i, i));
        }
    }
    
    (content.join("\n"), decorations)
}

fn benchmark_apply_decorations_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("apply_decorations_comparison");
    
    for size in [100, 1000, 5000, 10000].iter() {
        let (content, decorations) = generate_test_content_with_decorations(*size);
        let parser = Parser::new();
        
        // Benchmark original implementation
        group.bench_with_input(
            BenchmarkId::new("original", size),
            &(&content, &decorations),
            |b, (content, decorations)| {
                b.iter(|| {
                    parser.apply_decorations(
                        black_box(content), 
                        black_box(decorations), 
                        false
                    )
                });
            },
        );
        
        // Benchmark optimized implementation
        group.bench_with_input(
            BenchmarkId::new("optimized", size),
            &(&content, &decorations),
            |b, (content, decorations)| {
                b.iter(|| {
                    optimized::apply_decorations_optimized(
                        black_box(content), 
                        black_box(decorations), 
                        false
                    )
                });
            },
        );
        
        // Benchmark zero-copy implementation
        group.bench_with_input(
            BenchmarkId::new("zero_copy", size),
            &(&content, &decorations),
            |b, (content, decorations)| {
                b.iter(|| {
                    optimized::apply_decorations_zero_copy(
                        black_box(content), 
                        black_box(decorations), 
                        false
                    )
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_worst_case_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("worst_case_decorations");
    
    // Worst case: many decorations spread throughout the file
    let lines = 1000;
    let mut content = Vec::new();
    let mut decorations = Vec::new();
    
    for i in 0..lines {
        // Every 10th line has a decoration (100 decorations in 1000 lines)
        if i % 10 == 0 {
            content.push(format!(r#"let key_{} = "secret{}"; // @whiteout: "HIDDEN""#, i, i));
            decorations.push(Decoration::Inline {
                line: i + 1,
                local_value: format!(r#"let key_{} = "secret{}";"#, i, i),
                committed_value: "\"HIDDEN\"".to_string(),
            });
        } else {
            content.push(format!("let var_{} = {};", i, i));
        }
    }
    
    let content = content.join("\n");
    let parser = Parser::new();
    
    group.bench_function("original_100_decorations", |b| {
        b.iter(|| {
            parser.apply_decorations(
                black_box(&content), 
                black_box(&decorations), 
                false
            )
        });
    });
    
    group.bench_function("optimized_100_decorations", |b| {
        b.iter(|| {
            optimized::apply_decorations_optimized(
                black_box(&content), 
                black_box(&decorations), 
                false
            )
        });
    });
    
    group.finish();
}

criterion_group!(
    benches, 
    benchmark_apply_decorations_comparison,
    benchmark_worst_case_scenario
);
criterion_main!(benches);