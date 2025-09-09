// Optimized parser module with performance improvements
pub mod block;
pub mod inline;
pub mod partial;
pub mod simple;

use anyhow::Result;
use std::sync::Arc;
use once_cell::sync::Lazy;

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

// Cache parser instances to avoid regex recompilation
static PARSER_CACHE: Lazy<Arc<Parser>> = Lazy::new(|| {
    Arc::new(Parser::new())
});

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
    
    // Get cached instance for read-only operations
    pub fn cached() -> Arc<Parser> {
        PARSER_CACHE.clone()
    }

    // Optimized parse with capacity pre-allocation
    pub fn parse(&self, content: &str) -> Result<Vec<Decoration>> {
        // Pre-allocate based on heuristic (1 decoration per 50 lines)
        let estimated_capacity = content.lines().count() / 50;
        let mut decorations = Vec::with_capacity(estimated_capacity.max(4));
        
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

    // Optimized apply_decorations with string builder pattern
    pub fn apply_decorations(&self, content: &str, decorations: &[Decoration], use_local: bool) -> String {
        if decorations.is_empty() {
            return content.to_string();
        }
        
        let lines: Vec<&str> = content.lines().collect();
        let line_count = lines.len();
        
        // Pre-allocate result capacity based on content size
        let mut result = Vec::with_capacity(line_count);
        let mut skip_until = 0;
        
        // Create lookup maps for O(1) decoration access
        let mut inline_map = std::collections::HashMap::new();
        let mut block_map = std::collections::HashMap::new();
        let mut partial_map = std::collections::HashMap::new();
        
        for decoration in decorations {
            match decoration {
                Decoration::Inline { line, local_value, committed_value } => {
                    inline_map.insert(*line, (local_value, committed_value));
                }
                Decoration::Block { start_line, end_line, local_content, committed_content } => {
                    block_map.insert(*start_line, (*end_line, local_content, committed_content));
                }
                Decoration::Partial { line, replacements } => {
                    partial_map.insert(*line, replacements);
                }
            }
        }

        for (idx, line) in lines.iter().enumerate() {
            let line_num = idx + 1;
            
            if line_num <= skip_until {
                continue;
            }

            // Check for block decorations (most expensive, do first)
            if let Some((end_line, local_content, committed_content)) = block_map.get(&line_num) {
                if use_local {
                    // Smudge: Keep markers and show local content
                    result.push(line.to_string());
                    for content_line in local_content.lines() {
                        result.push(content_line.to_string());
                    }
                    if *end_line <= lines.len() {
                        result.push(lines[*end_line - 1].to_string());
                    }
                    skip_until = *end_line;
                    
                    let committed_lines = committed_content.lines().count();
                    if committed_lines > 0 {
                        skip_until += committed_lines;
                    }
                } else {
                    // Clean: Remove entire block including markers
                    if !committed_content.is_empty() {
                        for content_line in committed_content.lines() {
                            result.push(content_line.to_string());
                        }
                    }
                    skip_until = *end_line + committed_content.lines().count();
                }
                continue;
            }
            
            // Check for inline decorations
            if let Some((local_value, committed_value)) = inline_map.get(&line_num) {
                if use_local {
                    result.push(format!("{} // @whiteout: \"{}\"", local_value, committed_value));
                } else {
                    result.push(committed_value.to_string());
                }
                continue;
            }
            
            // Check for partial replacements
            if let Some(replacements) = partial_map.get(&line_num) {
                let mut processed_line = String::with_capacity(line.len() * 2);
                processed_line.push_str(line);
                
                // Apply replacements in reverse order to maintain indices
                for replacement in replacements.iter().rev() {
                    let new_value = if use_local {
                        format!("[[{}||{}]]", 
                            replacement.local_value, 
                            replacement.committed_value)
                    } else {
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
                continue;
            }
            
            // No decoration found, add line as-is
            result.push(line.to_string());
        }
        
        result.join("\n")
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}