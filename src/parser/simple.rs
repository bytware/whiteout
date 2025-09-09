use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;

use super::Decoration;

// Static regex compilation for performance
static PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^.*@whiteout.*$").expect("Failed to compile pattern")
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
            // Also skip @whiteout-start, @whiteout-end, and @whiteout: patterns
            if PATTERN.is_match(lines[i]) 
                && !lines[i].contains(r"\@whiteout")
                && !lines[i].contains("@whiteout-start")
                && !lines[i].contains("@whiteout-end")
                && !lines[i].contains("@whiteout:") {
                let start_line = i + 1;
                let mut local_lines = Vec::new();
                
                // The @whiteout line itself is the marker
                // Everything after it until a blank line or end of file is local content
                i += 1;
                
                while i < lines.len() && !lines[i].trim().is_empty() {
                    local_lines.push(lines[i]);
                    i += 1;
                }
                
                if !local_lines.is_empty() {
                    decorations.push(Decoration::Block {
                        start_line,
                        end_line: start_line + local_lines.len(),
                        local_content: local_lines.join("\n"),
                        committed_content: String::new(), // Nothing in committed version
                    });
                }
            }
            i += 1;
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
this too

normal again"#;
        
        let decorations = parser.parse(content).unwrap();
        assert_eq!(decorations.len(), 1);
        
        match &decorations[0] {
            Decoration::Block { local_content, committed_content, .. } => {
                assert!(local_content.contains("this will be hidden"));
                assert!(local_content.contains("this too"));
                assert!(committed_content.is_empty());
            }
            _ => panic!("Expected block decoration"),
        }
    }
}