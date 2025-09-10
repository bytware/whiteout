use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;

use super::Decoration;

// Static regex compilation for performance
// Match lines that have @whiteout as a standalone decoration (not part of other text)
static PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^\s*(?://|#|--|/\*|\*)\s*@whiteout\s*(?:\*/)?$").expect("Failed to compile pattern")
});

/// Parser for simple @whiteout lines that hide entire lines or blocks
pub struct SimpleParser;

impl Default for SimpleParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleParser {
    pub fn new() -> Self {
        // Force lazy static initialization
        let _ = &*PATTERN;
        Self
    }

    pub fn parse(&self, content: &str) -> Result<Vec<Decoration>> {
        let mut decorations = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        
        while i < lines.len() {
            // Check if line matches pattern and is not escaped
            // Also skip @whiteout-start, @whiteout-end, @whiteout:, and @whiteout-partial patterns
            if PATTERN.is_match(lines[i]) 
                && !lines[i].contains(r"\@whiteout")
                && !lines[i].contains("@whiteout-start")
                && !lines[i].contains("@whiteout-end")
                && !lines[i].contains("@whiteout:")
                && !lines[i].contains("@whiteout-partial") {
                let start_line = i + 1;
                
                // The @whiteout line itself is the marker
                // Only the next immediate line is local content
                i += 1;
                
                if i < lines.len() {
                    // Only capture the single next line
                    let local_content = lines[i].to_string();
                    
                    decorations.push(Decoration::Block {
                        start_line,
                        end_line: start_line + 1, // Only one line
                        local_content,
                        committed_content: String::new(), // Nothing in committed version
                    });
                    
                    i += 1; // Move past the hidden line
                }
                // Don't increment i here since we already did in the loop
            } else {
                i += 1;
            }
        }
        
        Ok(decorations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_parser() {
        let parser = SimpleParser::new();
        let content = r#"normal line
@whiteout
this will be hidden
this stays visible

normal again"#;
        
        let decorations = parser.parse(content).unwrap();
        assert_eq!(decorations.len(), 1);
        
        match &decorations[0] {
            Decoration::Block { local_content, committed_content, .. } => {
                assert_eq!(local_content, "this will be hidden");
                assert!(!local_content.contains("this stays visible"));
                assert!(committed_content.is_empty());
            }
            _ => panic!("Expected block decoration"),
        }
    }
}