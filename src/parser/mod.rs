pub mod apply;
pub mod block;
pub mod inline;
pub mod partial;
pub mod simple;
pub mod types;

use anyhow::Result;
pub use types::{Decoration, PartialReplacement};

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

    pub fn apply_decorations(
        &self,
        content: &str,
        decorations: &[Decoration],
        use_local: bool,
    ) -> String {
        apply::apply_decorations(content, decorations, use_local)
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
        let content = r#"let api_key = "sk-12345"; // @whiteout: "ENV_VAR""#;
        let decorations = parser.parse(content)?;
        
        assert_eq!(decorations.len(), 1);
        match &decorations[0] {
            Decoration::Inline { local_value, committed_value, .. } => {
                assert!(local_value.contains("sk-12345"));
                assert_eq!(committed_value, "\"ENV_VAR\"");
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
const DEBUG = false;"#;
        
        let decorations = parser.parse(content)?;
        assert_eq!(decorations.len(), 1);
        
        match &decorations[0] {
            Decoration::Block { local_content, committed_content, .. } => {
                assert!(local_content.contains("true"));
                assert!(committed_content.contains("false"));
            }
            _ => panic!("Expected block decoration"),
        }
        
        Ok(())
    }

    #[test]
    fn test_apply_decorations_clean() -> Result<()> {
        let parser = Parser::new();
        
        // Test inline decoration - preserves marker for smudge
        let content = r#"let api_key = "sk-12345"; // @whiteout: "REDACTED""#;
        let decorations = parser.parse(content)?;
        let cleaned = parser.apply_decorations(content, &decorations, false);
        assert_eq!(cleaned, "\"REDACTED\" // @whiteout: \"REDACTED\"");
        assert!(cleaned.contains("@whiteout"));  // Marker is preserved
        assert!(!cleaned.contains("sk-12345"));  // Secret is removed
        
        // Test block decoration - preserves markers for smudge
        let content = r#"code before
// @whiteout-start
const DEBUG = true;
// @whiteout-end
const DEBUG = false;
code after"#;
        let decorations = parser.parse(content)?;
        let cleaned = parser.apply_decorations(content, &decorations, false);
        assert!(cleaned.contains("code before"));
        assert!(cleaned.contains("// @whiteout-start"));  // Marker preserved
        assert!(cleaned.contains("// @whiteout-end"));    // Marker preserved  
        assert!(cleaned.contains("const DEBUG = false;"));
        assert!(cleaned.contains("code after"));
        assert!(!cleaned.contains("const DEBUG = true;"));  // Local content removed
        
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
    
    #[test]
    fn test_incomplete_block() -> Result<()> {
        let parser = Parser::new();
        
        // Test that incomplete blocks are left as-is
        let content = r#"
// @whiteout-start
const SECRET = "value";
// Missing @whiteout-end
const OTHER = "data";
"#;
        let decorations = parser.parse(content)?;
        // Should not find any decorations since block is incomplete
        if !decorations.is_empty() {
            eprintln!("Found {} decorations:", decorations.len());
            for (i, dec) in decorations.iter().enumerate() {
                eprintln!("  {}: {:?}", i, dec);
            }
        }
        assert_eq!(decorations.len(), 0);
        
        // When no decorations, apply_decorations should return content unchanged
        let result = parser.apply_decorations(content, &decorations, false);
        assert_eq!(result, content);
        
        Ok(())
    }
}