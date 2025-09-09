pub mod block;
pub mod inline;
pub mod partial;

use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum Decoration {
    Block {
        start_line: usize,
        end_line: usize,
        local_content: String,
        committed_content: String,
    },
    Inline {
        line: usize,
        local_value: String,
        committed_value: String,
    },
    Partial {
        line: usize,
        replacements: Vec<PartialReplacement>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartialReplacement {
    pub start: usize,
    pub end: usize,
    pub local_value: String,
    pub committed_value: String,
}

pub struct Parser {
    block_parser: block::BlockParser,
    inline_parser: inline::InlineParser,
    partial_parser: partial::PartialParser,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            block_parser: block::BlockParser::new(),
            inline_parser: inline::InlineParser::new(),
            partial_parser: partial::PartialParser::new(),
        }
    }

    pub fn parse(&self, content: &str) -> Result<Vec<Decoration>> {
        let mut decorations = Vec::new();
        
        decorations.extend(self.block_parser.parse(content)?);
        decorations.extend(self.inline_parser.parse(content)?);
        decorations.extend(self.partial_parser.parse(content)?);
        
        decorations.sort_by_key(|d| match d {
            Decoration::Block { start_line, .. } => *start_line,
            Decoration::Inline { line, .. } => *line,
            Decoration::Partial { line, .. } => *line,
        });
        
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
            
            for decoration in decorations {
                match decoration {
                    Decoration::Block { start_line, end_line, local_content, committed_content } => {
                        if line_num == *start_line {
                            if use_local {
                                result.push(local_content.clone());
                            } else {
                                result.push(committed_content.clone());
                            }
                            skip_until = *end_line;
                            line_processed = true;
                            break;
                        }
                    }
                    Decoration::Inline { line: dec_line, local_value, committed_value } => {
                        if line_num == *dec_line {
                            if use_local {
                                // Keep original line with local value
                                result.push(line.to_string());
                            } else {
                                // Replace local value with committed value, keep decoration
                                let original_line = line;
                                // The line format is: "local_value // @whiteout: committed_value"
                                // We want: "committed_value // @whiteout: committed_value"
                                if let Some(comment_idx) = original_line.find("// @whiteout:") {
                                    result.push(format!("{} {}", 
                                        committed_value.trim(), 
                                        &original_line[comment_idx..]));
                                } else {
                                    // Fallback
                                    result.push(format!("{} // @whiteout: {}", 
                                        committed_value.trim(), 
                                        committed_value.trim()));
                                }
                            }
                            line_processed = true;
                            break;
                        }
                    }
                    Decoration::Partial { line: dec_line, replacements } => {
                        if line_num == *dec_line {
                            let mut processed_line = line.to_string();
                            
                            for replacement in replacements.iter().rev() {
                                let value = if use_local { 
                                    &replacement.local_value 
                                } else { 
                                    &replacement.committed_value 
                                };
                                
                                if replacement.start < processed_line.len() {
                                    processed_line.replace_range(
                                        replacement.start..replacement.end.min(processed_line.len()),
                                        value
                                    );
                                }
                            }
                            
                            result.push(processed_line);
                            line_processed = true;
                            break;
                        }
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
    fn test_parser_creation() {
        let parser = Parser::new();
        let content = "let x = 5;";
        let decorations = parser.parse(content).unwrap();
        assert!(decorations.is_empty());
    }
}