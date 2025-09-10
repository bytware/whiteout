use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;

use super::{Decoration, PartialReplacement};

// Static regex compilation for performance
static PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[\[([^|]+)\|\|([^\]]+)\]\]").expect("Failed to compile pattern")
});

static DECORATOR_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"//\s*@whiteout-partial").expect("Failed to compile decorator pattern")
});

pub struct PartialParser;

impl Default for PartialParser {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialParser {
    pub fn new() -> Self {
        // Force lazy static initialization
        let _ = &*PATTERN;
        let _ = &*DECORATOR_PATTERN;
        Self
    }

    pub fn parse(&self, content: &str) -> Result<Vec<Decoration>> {
        let mut decorations = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            // Only process lines that have the @whiteout-partial decorator
            if !DECORATOR_PATTERN.is_match(line) {
                continue;
            }
            
            let mut replacements = Vec::new();
            
            for capture in PATTERN.captures_iter(line) {
                let match_pos = capture.get(0).unwrap();
                let local_value = capture.get(1).unwrap().as_str().to_string();
                let committed_value = capture.get(2).unwrap().as_str().to_string();
                
                replacements.push(PartialReplacement {
                    start: match_pos.start(),
                    end: match_pos.end(),
                    local_value,
                    committed_value,
                });
            }
            
            if !replacements.is_empty() {
                decorations.push(Decoration::Partial {
                    line: line_num + 1,
                    replacements,
                });
            }
        }
        
        Ok(decorations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partial_parser_with_decorator() {
        let parser = PartialParser::new();
        let content = r#"let url = "https://[[localhost:8080||api.example.com]]/v1"; // @whiteout-partial"#;
        
        let decorations = parser.parse(content).unwrap();
        assert_eq!(decorations.len(), 1);
        
        match &decorations[0] {
            Decoration::Partial { line, replacements } => {
                assert_eq!(*line, 1);
                assert_eq!(replacements.len(), 1);
                assert_eq!(replacements[0].local_value, "localhost:8080");
                assert_eq!(replacements[0].committed_value, "api.example.com");
            }
            _ => panic!("Expected partial decoration"),
        }
    }
    
    #[test]
    fn test_partial_parser_without_decorator_ignores() {
        let parser = PartialParser::new();
        // This should be ignored because it lacks @whiteout-partial
        let content = r#"let url = "https://[[localhost:8080||api.example.com]]/v1";"#;
        
        let decorations = parser.parse(content).unwrap();
        assert_eq!(decorations.len(), 0);
    }

    #[test]
    fn test_multiple_partial_replacements_with_decorator() {
        let parser = PartialParser::new();
        let content = r#"let config = { host: "[[dev.local||prod.com]]", port: [[8080||443]] }; // @whiteout-partial"#;
        
        let decorations = parser.parse(content).unwrap();
        assert_eq!(decorations.len(), 1);
        
        match &decorations[0] {
            Decoration::Partial { line, replacements } => {
                assert_eq!(*line, 1);
                assert_eq!(replacements.len(), 2);
            }
            _ => panic!("Expected partial decoration"),
        }
    }
    
    #[test]
    fn test_safe_from_accidental_matches() {
        let parser = PartialParser::new();
        // These should all be ignored without the decorator
        let content = r#"
// Markdown table: | Column [[A||B]] | Description |
let matrix = data[[row||col]];  // Array notation
let pattern = "[[a-z]||[0-9]]"; // Regex pattern
"#;
        
        let decorations = parser.parse(content).unwrap();
        assert_eq!(decorations.len(), 0);
    }
}