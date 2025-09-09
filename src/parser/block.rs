use anyhow::Result;
use regex::Regex;

use super::Decoration;

pub struct BlockParser {
    start_pattern: Regex,
    end_pattern: Regex,
}

impl BlockParser {
    pub fn new() -> Self {
        // Match @whiteout-start/end with any comment style or no comment at all
        let start_pattern = Regex::new(r"(?m)^.*@whiteout-start\s*$").unwrap();
        let end_pattern = Regex::new(r"(?m)^.*@whiteout-end\s*$").unwrap();
        
        Self {
            start_pattern,
            end_pattern,
        }
    }

    pub fn parse(&self, content: &str) -> Result<Vec<Decoration>> {
        let mut decorations = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        
        while i < lines.len() {
            // Check if line matches pattern and is not escaped
            if self.start_pattern.is_match(lines[i]) && !lines[i].contains(r"\@whiteout-start") {
                let start_line = i + 1;
                let mut local_lines = Vec::new();
                let mut committed_lines = Vec::new();
                
                i += 1;
                
                while i < lines.len() && !self.end_pattern.is_match(lines[i]) {
                    local_lines.push(lines[i]);
                    i += 1;
                }
                
                if i < lines.len() && self.end_pattern.is_match(lines[i]) {
                    let _end_marker_line = i + 1;
                    i += 1;
                    
                    while i < lines.len() {
                        if i + 1 < lines.len() && self.start_pattern.is_match(lines[i + 1]) {
                            break;
                        }
                        
                        if self.start_pattern.is_match(lines[i]) || self.end_pattern.is_match(lines[i]) {
                            break;
                        }
                        
                        committed_lines.push(lines[i]);
                        i += 1;
                        
                        if !committed_lines.is_empty() && 
                           (i >= lines.len() || lines[i].trim().is_empty() || 
                            self.start_pattern.is_match(lines[i])) {
                            break;
                        }
                    }
                    
                    // Push decoration even if local_lines is empty (for cleaned content)
                    decorations.push(Decoration::Block {
                        start_line,
                        end_line: start_line + local_lines.len() + 1, // end_line is the line with @whiteout-end
                        local_content: local_lines.join("\n"),
                        committed_content: committed_lines.join("\n"),
                    });
                }
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
    fn test_block_parser() {
        let parser = BlockParser::new();
        let content = r#"
// @whiteout-start
const DEBUG = true;
const LOG_LEVEL = "trace";
// @whiteout-end
const DEBUG = false;
const LOG_LEVEL = "error";
"#;
        
        let decorations = parser.parse(content).unwrap();
        assert_eq!(decorations.len(), 1);
        
        match &decorations[0] {
            Decoration::Block { start_line, end_line, local_content, committed_content } => {
                assert_eq!(*start_line, 2);
                assert!(local_content.contains("true"));
                assert!(committed_content.contains("false"));
            }
            _ => panic!("Expected block decoration"),
        }
    }

    #[test]
    fn test_multiple_blocks() {
        let parser = BlockParser::new();
        let content = r#"
// @whiteout-start
let x = 1;
// @whiteout-end
let x = 2;

// @whiteout-start
let y = 3;
// @whiteout-end
let y = 4;
"#;
        
        let decorations = parser.parse(content).unwrap();
        assert_eq!(decorations.len(), 2);
    }
}