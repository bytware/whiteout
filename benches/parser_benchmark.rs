use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use whiteout::parser::Parser;

fn generate_test_content(lines: usize, decorations_per_100_lines: usize) -> String {
    let mut content = Vec::new();
    
    for i in 0..lines {
        if i % (100 / decorations_per_100_lines) == 0 {
            // Add inline decoration
            content.push(format!(
                r#"let api_key_{} = "sk-12345-{}"; // @whiteout: "REDACTED""#, 
                i, i
            ));
        } else if i % (100 / decorations_per_100_lines) == 50 {
            // Add block decoration
            content.push(format!("// @whiteout-start"));
            content.push(format!("const DEBUG_{} = true;", i));
            content.push(format!("// @whiteout-end"));
            content.push(format!("const DEBUG_{} = false;", i));
        } else {
            // Regular code line
            content.push(format!("let var_{} = {};", i, i));
        }
    }
    
    content.join("\n")
}

fn benchmark_parse(c: &mut Criterion) {
    let parser = Parser::new();
    let mut group = c.benchmark_group("parser_parse");
    
    for size in [100, 1000, 5000, 10000].iter() {
        let content = generate_test_content(*size, 10);
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &content,
            |b, content| {
                b.iter(|| {
                    parser.parse(black_box(content)).unwrap()
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_apply_decorations(c: &mut Criterion) {
    let parser = Parser::new();
    let mut group = c.benchmark_group("parser_apply");
    
    for size in [100, 1000, 5000].iter() {
        let content = generate_test_content(*size, 10);
        let decorations = parser.parse(&content).unwrap();
        
        group.bench_with_input(
            BenchmarkId::new("clean", size),
            &(content.clone(), decorations.clone()),
            |b, (content, decorations)| {
                b.iter(|| {
                    parser.apply_decorations(black_box(content), black_box(decorations), false)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("smudge", size),
            &(content.clone(), decorations.clone()),
            |b, (content, decorations)| {
                b.iter(|| {
                    parser.apply_decorations(black_box(content), black_box(decorations), true)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_regex_compilation(c: &mut Criterion) {
    use regex::Regex;
    
    c.bench_function("regex_compile_inline", |b| {
        b.iter(|| {
            Regex::new(r"(?m)^(.+?)\s*(?://|#|--)\s*@whiteout:\s*(.+?)$").unwrap()
        });
    });
    
    c.bench_function("regex_compile_block", |b| {
        b.iter(|| {
            let _start = Regex::new(r"(?m)^.*@whiteout-start\s*$").unwrap();
            let _end = Regex::new(r"(?m)^.*@whiteout-end\s*$").unwrap();
        });
    });
}

criterion_group!(benches, benchmark_parse, benchmark_apply_decorations, benchmark_regex_compilation);
criterion_main!(benches);