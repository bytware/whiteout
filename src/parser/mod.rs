pub mod block;
pub mod inline;
pub mod partial;
pub mod simple;

use anyhow::Result;

#[derive(Debug, Clone)]
pub enum Decoration {
    Inline {
        line: usize,
        local_value: String,
        committed_value: String,
    },
    Block {
        start_line: usize,
        end_line: usize,
        local_content: String,
        committed_content: String,
    },
    Partial {
        line: usize,
        replacements: Vec<PartialReplacement>,
    },
}

#[derive(Debug, Clone)]
pub struct PartialReplacement {
    pub start: usize,
    pub end: usize,
    pub local_value: String,
    pub committed_value: String,
}

pub struct Parser {
    inline_parser: inline::InlineParser,
    block_parser: block::BlockParser,
    partial_parser: partial::PartialParser,
    simple_parser: simple::SimpleParser,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            inline_parser: inline::InlineParser::new(),
            block_parser: block::BlockParser::new(),
            partial_parser: partial::PartialParser::new(),
            simple_parser: simple::SimpleParser::new(),
        }
    }

    pub fn parse(&self, content: &str) -> Result<Vec<Decoration>> {
        let mut decorations = Vec::new();
        
        // Parse simple @whiteout decorations first
        decorations.extend(self.simple_parser.parse(content)?);
        
        // Parse inline decorations
        decorations.extend(self.inline_parser.parse(content)?);
        
        // Parse block decorations
        decorations.extend(self.block_parser.parse(content)?);
        
        // Parse partial replacements
        decorations.extend(self.partial_parser.parse(content)?);
        
        Ok(decorations)
    }

    pub fn apply_decorations(&self, content: &str, decorations: &[Decoration], use_local: bool) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();
        let mut skip_until = 0;

        for (idx, line) in lines.iter().enumerate() {
            let line_num = idx + 1;
            
            if line_num <= skip_until {
                continue;
            }

            let mut line_processed = false;
            
            // Check for block decorations
            for decoration in decorations {
                if let Decoration::Block { start_line, end_line, local_content, committed_content } = decoration {
                    if line_num == *start_line {
                        if use_local {
                            // Smudge: Keep markers and show local content
                            result.push(line.to_string()); // Keep @whiteout-start
                            for content_line in local_content.lines() {
                                result.push(content_line.to_string());
                            }
                            // Find and add the end marker
                            if *end_line <= lines.len() {
                                result.push(lines[*end_line - 1].to_string()); // Keep @whiteout-end
                            }
                            // Skip the committed content that follows
                            skip_until = *end_line;
                            
                            // Count lines of committed content to skip
                            let committed_lines = committed_content.lines().count();
                            if committed_lines > 0 {
                                skip_until += committed_lines;
                            }
                        } else {
                            // Clean: Remove entire block including markers, show only committed content
                            if !committed_content.is_empty() {
                                for content_line in committed_content.lines() {
                                    result.push(content_line.to_string());
                                }
                            }
                            // Skip to end of block plus the committed content that follows
                            skip_until = *end_line + committed_content.lines().count();
                        }
                        line_processed = true;
                        break;
                    }
                }
            }
            
            if line_processed {
                continue;
            }
            
            // Check for inline decorations
            let mut found_inline = false;
            for decoration in decorations {
                if let Decoration::Inline { line: dec_line, local_value, committed_value } = decoration {
                    if line_num == *dec_line {
                        if use_local {
                            // Smudge: Show local value with decoration
                            result.push(format!("{} // @whiteout: \"{}\"", local_value, committed_value));
                        } else {
                            // Clean: Show only committed value without decoration
                            result.push(committed_value.to_string());
                        }
                        found_inline = true;
                        line_processed = true;
                        break;
                    }
                }
            }
            
            if found_inline {
                continue;
            }
            
            // Check for partial replacements
            for decoration in decorations {
                if let Decoration::Partial { line: dec_line, replacements } = decoration {
                    if line_num == *dec_line {
                        let mut processed_line = line.to_string();
                        
                        for replacement in replacements.iter().rev() {
                            let new_value = if use_local {
                                // Smudge: Use local value in the pattern
                                format!("[[{}||{}]]", 
                                    replacement.local_value, 
                                    replacement.committed_value)
                            } else {
                                // Clean: Replace entire pattern with just committed value
                                replacement.committed_value.clone()
                            };
                            
                            if replacement.start < processed_line.len() {
                                processed_line.replace_range(
                                    replacement.start..replacement.end.min(processed_line.len()),
                                    &new_value
                                );
                            }
                        }
                        
                        result.push(processed_line);
                        line_processed = true;
                        break;
                    }
                }
            }
            
            if !line_processed {
                result.push(line.to_string());
            }
        }
        
        result.join("\n")
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_inline() -> Result<()> {
        let parser = Parser::new();
        let content = r#"let api_key = "sk-12345"; // @whiteout: "REDACTED""#;
        
        let decorations = parser.parse(content)?;
        assert_eq!(decorations.len(), 1);
        
        match &decorations[0] {
            Decoration::Inline { local_value, committed_value, .. } => {
                assert_eq!(local_value, r#"let api_key = "sk-12345";"#);
                assert_eq!(committed_value, "\"REDACTED\"");
            }
            _ => panic!("Expected inline decoration"),
        }
        
        Ok(())
    }

    #[test]
    fn test_parse_block() -> Result<()> {
        let parser = Parser::new();
        let content = r#"
// @whiteout-start
const DEBUG = true;
// @whiteout-end
const DEBUG = false;
"#;
        
        let decorations = parser.parse(content)?;
        assert_eq!(decorations.len(), 1);
        
        match &decorations[0] {
            Decoration::Block { local_content, committed_content, .. } => {
                assert_eq!(local_content.trim(), "const DEBUG = true;");
                assert_eq!(committed_content.trim(), "const DEBUG = false;");
            }
            _ => panic!("Expected block decoration"),
        }
        
        Ok(())
    }

    #[test]
    fn test_apply_decorations_clean() -> Result<()> {
        let parser = Parser::new();
        
        // Test inline decoration removal
        let content = r#"let api_key = "sk-12345"; // @whiteout: "REDACTED""#;
        let decorations = parser.parse(content)?;
        let cleaned = parser.apply_decorations(content, &decorations, false);
        assert_eq!(cleaned, "\"REDACTED\"");
        assert!(!cleaned.contains("@whiteout"));
        assert!(!cleaned.contains("sk-12345"));
        
        // Test block decoration removal
        let content = r#"code before
// @whiteout-start
const DEBUG = true;
// @whiteout-end
const DEBUG = false;
code after"#;
        let decorations = parser.parse(content)?;
        let cleaned = parser.apply_decorations(content, &decorations, false);
        assert!(cleaned.contains("code before"));
        assert!(cleaned.contains("const DEBUG = false;"));
        assert!(cleaned.contains("code after"));
        assert!(!cleaned.contains("@whiteout"));
        assert!(!cleaned.contains("const DEBUG = true;"));
        
        Ok(())
    }

    #[test]
    fn test_apply_decorations_smudge() -> Result<()> {
        let parser = Parser::new();
        
        // Test inline decoration preservation
        let content = r#""REDACTED" // @whiteout: "REDACTED""#;
        let decorations = vec![Decoration::Inline {
            line: 1,
            local_value: r#"let api_key = "sk-12345";"#.to_string(),
            committed_value: "\"REDACTED\"".to_string(),
        }];
        let smudged = parser.apply_decorations(content, &decorations, true);
        assert!(smudged.contains("sk-12345"));
        assert!(smudged.contains("@whiteout"));
        
        Ok(())
    }
}