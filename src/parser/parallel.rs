use anyhow::Result;
use rayon::prelude::*;
use aho_corasick::AhoCorasick;
use once_cell::sync::Lazy;

use super::{Decoration, PartialReplacement};
use super::inline::InlineParser;
use super::block::BlockParser;
use super::partial::PartialParser;
use super::simple::SimpleParser;

/// Pre-filter patterns for quick rejection of lines without decorations
static DECORATION_PATTERNS: Lazy<AhoCorasick> = Lazy::new(|| {
    AhoCorasick::new(&[
        "@whiteout:",
        "@whiteout-start",
        "@whiteout-end",
        "@whiteout",
        "[[",
        "||",
        "]]"
    ]).expect("Failed to compile Aho-Corasick patterns")
});

/// Parallel parser that processes lines concurrently for better performance
/// on multi-core systems
pub struct ParallelParser {
    inline_parser: InlineParser,
    block_parser: BlockParser,
    partial_parser: PartialParser,
    simple_parser: SimpleParser,
}

impl ParallelParser {
    pub fn new() -> Self {
        // Force lazy static initialization
        let _ = &*DECORATION_PATTERNS;
        
        Self {
            inline_parser: InlineParser::new(),
            block_parser: BlockParser::new(),
            partial_parser: PartialParser::new(),
            simple_parser: SimpleParser::new(),
        }
    }

    /// Parse content using parallel processing for improved performance
    pub fn parse(&self, content: &str) -> Result<Vec<Decoration>> {
        // Quick check: if no decoration patterns found, return early
        if !DECORATION_PATTERNS.is_match(content) {
            return Ok(Vec::new());
        }

        // Split content into chunks for parallel processing
        let lines: Vec<&str> = content.lines().collect();
        let chunk_size = (lines.len() / rayon::current_num_threads()).max(100);
        
        // Process inline and partial decorations in parallel
        let inline_and_partial: Vec<Decoration> = lines
            .par_chunks(chunk_size)
            .enumerate()
            .flat_map(|(chunk_idx, chunk)| {
                let mut local_decorations = Vec::new();
                let base_line = chunk_idx * chunk_size;
                
                for (idx, line) in chunk.iter().enumerate() {
                    let line_num = base_line + idx + 1;
                    
                    // Use Aho-Corasick for fast pre-filtering
                    if !DECORATION_PATTERNS.is_match(line) {
                        continue;
                    }
                    
                    // Check for inline decoration
                    if line.contains("@whiteout:") && !line.contains(r"\@whiteout:") {
                        if let Ok(decorations) = self.parse_inline_single(line, line_num) {
                            local_decorations.extend(decorations);
                        }
                    }
                    
                    // Check for partial replacements
                    if line.contains("[[") && line.contains("||") && line.contains("]]") {
                        if let Ok(decorations) = self.parse_partial_single(line, line_num) {
                            local_decorations.extend(decorations);
                        }
                    }
                }
                
                local_decorations
            })
            .collect();

        // Block decorations need sequential processing due to state dependency
        // But we can still optimize with pre-filtering
        let block_decorations = self.parse_blocks_optimized(content)?;
        
        // Simple decorations
        let simple_decorations = self.simple_parser.parse(content)?;
        
        // Combine all decorations
        let mut all_decorations = Vec::with_capacity(
            inline_and_partial.len() + block_decorations.len() + simple_decorations.len()
        );
        all_decorations.extend(simple_decorations);
        all_decorations.extend(inline_and_partial);
        all_decorations.extend(block_decorations);
        
        Ok(all_decorations)
    }

    fn parse_inline_single(&self, line: &str, line_num: usize) -> Result<Vec<Decoration>> {
        use regex::Regex;
        use once_cell::sync::Lazy;
        
        static INLINE_PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"^(.+?)\s*(?://|#|--)\s*@whiteout:\s*(.+?)$")
                .expect("Failed to compile inline pattern")
        });
        
        let mut decorations = Vec::new();
        
        if let Some(captures) = INLINE_PATTERN.captures(line) {
            let local_value = captures.get(1).unwrap().as_str().trim().to_string();
            let committed_value = captures.get(2).unwrap().as_str().trim().to_string();
            
            decorations.push(Decoration::Inline {
                line: line_num,
                local_value,
                committed_value,
            });
        }
        
        Ok(decorations)
    }

    fn parse_partial_single(&self, line: &str, line_num: usize) -> Result<Vec<Decoration>> {
        use regex::Regex;
        use once_cell::sync::Lazy;
        
        static PARTIAL_PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"\[\[([^\|]+)\|\|([^\]]+)\]\]")
                .expect("Failed to compile partial pattern")
        });
        
        let mut replacements = Vec::new();
        
        for capture in PARTIAL_PATTERN.captures_iter(line) {
            let full_match = capture.get(0).unwrap();
            let local_value = capture.get(1).unwrap().as_str().to_string();
            let committed_value = capture.get(2).unwrap().as_str().to_string();
            
            replacements.push(PartialReplacement {
                start: full_match.start(),
                end: full_match.end(),
                local_value,
                committed_value,
            });
        }
        
        if !replacements.is_empty() {
            return Ok(vec![Decoration::Partial {
                line: line_num,
                replacements,
            }]);
        }
        
        Ok(Vec::new())
    }

    fn parse_blocks_optimized(&self, content: &str) -> Result<Vec<Decoration>> {
        // Use Aho-Corasick to quickly find potential block locations
        let block_patterns = AhoCorasick::new(&["@whiteout-start", "@whiteout-end"])
            .expect("Failed to compile block patterns");
        
        // Quick check
        if !block_patterns.is_match(content) {
            return Ok(Vec::new());
        }
        
        // Fall back to sequential block parser for correctness
        self.block_parser.parse(content)
    }
}

impl Default for ParallelParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Parallel file processor for handling multiple files concurrently
pub fn process_files_parallel<P: AsRef<Path> + Sync>(
    file_paths: &[P],
    processor: impl Fn(&Path) -> Result<()> + Sync,
) -> Vec<Result<()>> {
    file_paths
        .par_iter()
        .map(|path| processor(path.as_ref()))
        .collect()
}

use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_parser() -> Result<()> {
        let parser = ParallelParser::new();
        
        let content = r#"
let api_key = "sk-12345"; // @whiteout: "REDACTED"
let normal = 123;
// @whiteout-start
const DEBUG = true;
// @whiteout-end
const DEBUG = false;
let url = "[[http://localhost||https://api.com]]";
"#;
        
        let decorations = parser.parse(content)?;
        assert!(decorations.len() >= 3); // At least inline, block, and partial
        
        Ok(())
    }

    #[test]
    fn test_early_rejection() -> Result<()> {
        let parser = ParallelParser::new();
        
        // Content with no decoration patterns
        let content = r#"
fn main() {
    let x = 1;
    let y = 2;
    println!("Hello, world!");
}
"#;
        
        let decorations = parser.parse(content)?;
        assert_eq!(decorations.len(), 0);
        
        Ok(())
    }

    #[test]
    fn test_parallel_performance() -> Result<()> {
        let parser = ParallelParser::new();
        
        // Generate large content
        let mut lines = Vec::new();
        for i in 0..10000 {
            if i % 100 == 0 {
                lines.push(format!(r#"let key_{} = "secret{}"; // @whiteout: "HIDDEN""#, i, i));
            } else {
                lines.push(format!("let var_{} = {};", i, i));
            }
        }
        let content = lines.join("\n");
        
        let decorations = parser.parse(&content)?;
        assert_eq!(decorations.len(), 100); // 100 inline decorations
        
        Ok(())
    }
}