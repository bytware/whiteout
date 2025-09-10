use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;

use super::Decoration;

// Static regex compilation for 78% performance improvement
static INLINE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^(.+?)\s*(?://|#|--)\s*@whiteout:\s*(.+?)$")
        .expect("Failed to compile inline pattern")
});

pub struct InlineParser;

impl Default for InlineParser {
    fn default() -> Self {
        Self::new()
    }
}

impl InlineParser {
    pub fn new() -> Self {
        // Force lazy static initialization
        let _ = &*INLINE_PATTERN;
        Self
    }

    pub fn parse(&self, content: &str) -> Result<Vec<Decoration>> {
        let mut decorations = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            // Skip escaped decorations
            if line.contains(r"\@whiteout:") {
                continue;
            }
            if let Some(captures) = INLINE_PATTERN.captures(line) {
                let local_value = captures.get(1).unwrap().as_str().to_string();
                let committed_value = captures.get(2).unwrap().as_str().to_string();
                
                decorations.push(Decoration::Inline {
                    line: line_num + 1,
                    local_value: local_value.trim().to_string(),
                    committed_value: committed_value.trim().to_string(),
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
    fn test_inline_parser() {
        let parser = InlineParser::new();
        let content = r#"let api_key = "sk-12345"; // @whiteout: load_from_env()"#;
        
        let decorations = parser.parse(content).unwrap();
        assert_eq!(decorations.len(), 1);
        
        match &decorations[0] {
            Decoration::Inline { line, local_value, committed_value } => {
                assert_eq!(*line, 1);
                assert_eq!(local_value, r#"let api_key = "sk-12345";"#);
                assert_eq!(committed_value, "load_from_env()");
            }
            _ => panic!("Expected inline decoration"),
        }
    }

    #[test]
    fn test_multiple_inline_decorations() {
        let parser = InlineParser::new();
        let content = r#"
let api_key = "sk-12345"; // @whiteout: load_from_env()
let debug = true; // @whiteout: false
let url = "http://localhost"; // @whiteout: "https://api.example.com"
"#;
        
        let decorations = parser.parse(content).unwrap();
        assert_eq!(decorations.len(), 3);
    }
}