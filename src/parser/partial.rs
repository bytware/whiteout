use anyhow::Result;
use regex::Regex;

use super::{Decoration, PartialReplacement};

pub struct PartialParser {
    pattern: Regex,
}

impl PartialParser {
    pub fn new() -> Self {
        let pattern = Regex::new(r"\[\[([^|]+)\|\|([^\]]+)\]\]").unwrap();
        
        Self { pattern }
    }

    pub fn parse(&self, content: &str) -> Result<Vec<Decoration>> {
        let mut decorations = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            let mut replacements = Vec::new();
            
            for capture in self.pattern.captures_iter(line) {
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
    fn test_partial_parser() {
        let parser = PartialParser::new();
        let content = r#"let url = "https://[[localhost:8080||api.example.com]]/v1";"#;
        
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
    fn test_multiple_partial_replacements() {
        let parser = PartialParser::new();
        let content = r#"let config = { host: "[[dev.local||prod.com]]", port: [[8080||443]] };"#;
        
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
}