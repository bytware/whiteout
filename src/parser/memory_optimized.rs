use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use std::borrow::Cow;

/// Memory-optimized decoration type using Cow for zero-copy operations
#[derive(Debug, Clone)]
pub enum OptimizedDecoration<'a> {
    Inline {
        line: usize,
        local_value: Cow<'a, str>,
        committed_value: Cow<'a, str>,
    },
    Block {
        start_line: usize,
        end_line: usize,
        local_content: Cow<'a, str>,
        committed_content: Cow<'a, str>,
    },
    Partial {
        line: usize,
        replacements: Vec<PartialReplacement<'a>>,
    },
    Simple {
        line: usize,
        hidden_content: Cow<'a, str>,
    },
}

#[derive(Debug, Clone)]
pub struct PartialReplacement<'a> {
    pub local_value: Cow<'a, str>,
    pub committed_value: Cow<'a, str>,
    pub start_col: usize,
    pub end_col: usize,
}

// Static regex patterns for performance
static INLINE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^(.+?)\s*(?://|#|--)\s*@whiteout:\s*(.+?)$")
        .expect("Failed to compile inline pattern")
});

static BLOCK_START_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^.*@whiteout-start\s*$").expect("Failed to compile block start pattern")
});

static BLOCK_END_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^.*@whiteout-end\s*$").expect("Failed to compile block end pattern")
});

static PARTIAL_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[\[([^|]+)\|\|([^\]]+)\]\]").expect("Failed to compile partial pattern")
});

/// Memory-optimized parser that minimizes allocations
pub struct OptimizedParser;

impl OptimizedParser {
    pub fn new() -> Self {
        // Force lazy static initialization
        let _ = &*INLINE_PATTERN;
        let _ = &*BLOCK_START_PATTERN;
        let _ = &*BLOCK_END_PATTERN;
        let _ = &*PARTIAL_PATTERN;
        Self
    }
    
    /// Parse content with minimal memory allocations
    pub fn parse<'a>(&self, content: &'a str) -> Result<Vec<OptimizedDecoration<'a>>> {
        let mut decorations = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        // Pre-allocate with estimated capacity
        decorations.reserve(lines.len() / 20); // Assume ~5% of lines have decorations
        
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            let line_num = i + 1;
            
            // Skip escaped decorations
            if line.contains(r"\@whiteout") {
                i += 1;
                continue;
            }
            
            // Check for inline decoration
            if let Some(captures) = INLINE_PATTERN.captures(line) {
                let local_match = captures.get(1).unwrap();
                let committed_match = captures.get(2).unwrap();
                
                // Use Cow::Borrowed to avoid allocation when possible
                decorations.push(OptimizedDecoration::Inline {
                    line: line_num,
                    local_value: Cow::Borrowed(local_match.as_str().trim()),
                    committed_value: Cow::Borrowed(committed_match.as_str().trim()),
                });
            }
            
            // Check for block start
            else if BLOCK_START_PATTERN.is_match(line) {
                let start_line = line_num;
                let mut local_lines = Vec::new();
                
                i += 1;
                while i < lines.len() && !BLOCK_END_PATTERN.is_match(lines[i]) {
                    local_lines.push(lines[i]);
                    i += 1;
                }
                
                if i < lines.len() {
                    let end_line = i + 1;
                    i += 1;
                    
                    // Collect committed content
                    let mut committed_lines = Vec::new();
                    while i < lines.len() {
                        let next_line = lines[i];
                        if next_line.trim().starts_with("//") {
                            let content = next_line.trim_start_matches('/').trim();
                            committed_lines.push(content);
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    
                    // Only allocate if we need to join multiple lines
                    let local_content = if local_lines.len() == 1 {
                        Cow::Borrowed(local_lines[0])
                    } else {
                        Cow::Owned(local_lines.join("\n"))
                    };
                    
                    let committed_content = if committed_lines.len() == 1 {
                        Cow::Borrowed(committed_lines[0])
                    } else if committed_lines.is_empty() {
                        Cow::Borrowed("")
                    } else {
                        Cow::Owned(committed_lines.join("\n"))
                    };
                    
                    decorations.push(OptimizedDecoration::Block {
                        start_line,
                        end_line,
                        local_content,
                        committed_content,
                    });
                    
                    continue;
                }
            }
            
            // Check for partial replacements
            else if line.contains("[[") && line.contains("||") && line.contains("]]") {
                let mut replacements = Vec::new();
                
                for capture in PARTIAL_PATTERN.captures_iter(line) {
                    let full_match = capture.get(0).unwrap();
                    let local_match = capture.get(1).unwrap();
                    let committed_match = capture.get(2).unwrap();
                    
                    replacements.push(PartialReplacement {
                        local_value: Cow::Borrowed(local_match.as_str()),
                        committed_value: Cow::Borrowed(committed_match.as_str()),
                        start_col: full_match.start(),
                        end_col: full_match.end(),
                    });
                }
                
                if !replacements.is_empty() {
                    decorations.push(OptimizedDecoration::Partial {
                        line: line_num,
                        replacements,
                    });
                }
            }
            
            // Check for simple decoration
            else if line.contains("@whiteout") &&
                    !line.contains("@whiteout:") &&
                    !line.contains("@whiteout-") {
                decorations.push(OptimizedDecoration::Simple {
                    line: line_num,
                    hidden_content: Cow::Borrowed(line),
                });
            }
            
            i += 1;
        }
        
        // Shrink to fit to release excess capacity
        decorations.shrink_to_fit();
        
        Ok(decorations)
    }
    
    /// Apply decorations to content with minimal allocations
    pub fn apply_clean<'a>(
        &self,
        content: &'a str,
        decorations: &[OptimizedDecoration<'a>],
    ) -> Cow<'a, str> {
        if decorations.is_empty() {
            return Cow::Borrowed(content);
        }
        
        // Pre-calculate if we need to modify the content
        let needs_modification = decorations.iter().any(|d| {
            !matches!(d, OptimizedDecoration::Simple { .. })
        });
        
        if !needs_modification {
            // If we only have Simple decorations that hide lines,
            // we might still return borrowed content
            return Cow::Borrowed(content);
        }
        
        let lines: Vec<&str> = content.lines().collect();
        let mut result = String::with_capacity(content.len());
        
        for (idx, line) in lines.iter().enumerate() {
            let line_num = idx + 1;
            let mut line_modified = false;
            let mut skip_line = false;
            
            for decoration in decorations {
                match decoration {
                    OptimizedDecoration::Inline { line: dec_line, local_value, committed_value } 
                        if *dec_line == line_num => {
                        if let Some(pos) = line.find(local_value.as_ref()) {
                            result.push_str(&line[..pos]);
                            result.push_str(committed_value.as_ref());
                            let end_pos = pos + local_value.len();
                            if end_pos < line.len() {
                                result.push_str(&line[end_pos..]);
                            }
                            line_modified = true;
                            break;
                        }
                    }
                    OptimizedDecoration::Simple { line: dec_line, .. } 
                        if *dec_line == line_num => {
                        skip_line = true;
                        break;
                    }
                    _ => {}
                }
            }
            
            if !skip_line && !line_modified {
                result.push_str(line);
            }
            
            if idx < lines.len() - 1 && !skip_line {
                result.push('\n');
            }
        }
        
        Cow::Owned(result)
    }
}

impl Default for OptimizedParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_zero_copy_for_unchanged_content() {
        let parser = OptimizedParser::new();
        let content = "no decorations here\njust plain text";
        
        let decorations = parser.parse(content).unwrap();
        assert!(decorations.is_empty());
        
        let result = parser.apply_clean(content, &decorations);
        assert!(matches!(result, Cow::Borrowed(_)));
    }
    
    #[test]
    fn test_minimal_allocations_for_inline() {
        let parser = OptimizedParser::new();
        let content = "let key = \"secret\"; // @whiteout: \"REDACTED\"";
        
        let decorations = parser.parse(content).unwrap();
        assert_eq!(decorations.len(), 1);
        
        match &decorations[0] {
            OptimizedDecoration::Inline { local_value, committed_value, .. } => {
                assert!(matches!(local_value, Cow::Borrowed(_)));
                assert!(matches!(committed_value, Cow::Borrowed(_)));
            }
            _ => panic!("Expected inline decoration"),
        }
    }
    
    #[test]
    fn test_memory_efficiency_for_large_files() {
        let parser = OptimizedParser::new();
        
        // Create a large file with sparse decorations
        let mut lines = Vec::new();
        for i in 0..10000 {
            if i % 100 == 0 {
                lines.push(format!("value_{} // @whiteout: \"REDACTED\"", i));
            } else {
                lines.push(format!("normal line {}", i));
            }
        }
        let content = lines.join("\n");
        
        let start = std::time::Instant::now();
        let decorations = parser.parse(&content).unwrap();
        let parse_time = start.elapsed();
        
        assert_eq!(decorations.len(), 100);
        assert!(parse_time.as_millis() < 50); // Should be very fast
        
        // Most values should be borrowed, not owned
        let borrowed_count = decorations.iter().filter(|d| {
            match d {
                OptimizedDecoration::Inline { local_value, committed_value, .. } => {
                    matches!(local_value, Cow::Borrowed(_)) && 
                    matches!(committed_value, Cow::Borrowed(_))
                }
                _ => false,
            }
        }).count();
        
        assert_eq!(borrowed_count, 100);
    }
}