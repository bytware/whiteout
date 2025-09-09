// Optimized inline parser with cached regex and streaming parsing
use anyhow::Result;
use regex::Regex;
use once_cell::sync::Lazy;

use super::Decoration;

// Cache compiled regex patterns
static INLINE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^(.+?)\s*(?://|#|--)\s*@whiteout:\s*(.+?)$")
        .expect("Invalid inline pattern regex")
});

pub struct InlineParser {
    pattern: &'static Regex,
}

impl InlineParser {
    pub fn new() -> Self {
        Self { 
            pattern: &INLINE_PATTERN,
        }
    }

    pub fn parse(&self, content: &str) -> Result<Vec<Decoration>> {
        // Pre-allocate with reasonable capacity
        let mut decorations = Vec::with_capacity(content.lines().count() / 100);
        
        // Use enumerate with pre-check for performance
        for (line_num, line) in content.lines().enumerate() {
            // Quick rejection checks before regex
            if line.len() < 15 { // Minimum viable decoration length
                continue;
            }
            
            // Skip escaped decorations
            if line.contains(r"\@whiteout:") {
                continue;
            }
            
            // Check for decoration marker before running regex
            if !line.contains("@whiteout:") {
                continue;
            }
            
            if let Some(captures) = self.pattern.captures(line) {
                let local_value = captures.get(1).unwrap().as_str();
                let committed_value = captures.get(2).unwrap().as_str();
                
                decorations.push(Decoration::Inline {
                    line: line_num + 1,
                    local_value: local_value.trim().to_string(),
                    committed_value: committed_value.trim().to_string(),
                });
            }
        }
        
        // Shrink to fit to release excess capacity
        decorations.shrink_to_fit();
        Ok(decorations)
    }
    
    // Streaming parse for large files
    pub fn parse_streaming<R: std::io::BufRead>(&self, reader: R) -> Result<Vec<Decoration>> {
        let mut decorations = Vec::new();
        
        for (line_num, line_result) in reader.lines().enumerate() {
            let line = line_result?;
            
            if line.len() < 15 || line.contains(r"\@whiteout:") || !line.contains("@whiteout:") {
                continue;
            }
            
            if let Some(captures) = self.pattern.captures(&line) {
                let local_value = captures.get(1).unwrap().as_str();
                let committed_value = captures.get(2).unwrap().as_str();
                
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