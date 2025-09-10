use anyhow::Result;
use std::path::Path;

use crate::{
    config::Config,
    parser::{Decoration, Parser},
    storage::LocalStorage,
};

pub fn apply(
    content: &str,
    file_path: &Path,
    storage: &LocalStorage,
    _config: &Config,
) -> Result<String> {
    // Clean filter stores local values and returns content with committed values
    // but preserves decoration markers so smudge can work later
    
    let parser = Parser::new();
    let decorations = parser.parse(content)?;
    
    if decorations.is_empty() {
        return Ok(content.to_string());
    }
    
    // Store all local values
    for decoration in &decorations {
        match decoration {
            Decoration::Inline { line, local_value, .. } => {
                storage.store_value(
                    file_path,
                    &format!("inline_{}", line),
                    local_value,
                )?;
            }
            Decoration::Block { start_line, local_content, .. } => {
                storage.store_value(
                    file_path,
                    &format!("block_{}", start_line),
                    local_content,
                )?;
            }
            Decoration::Partial { line, replacements } => {
                for (idx, replacement) in replacements.iter().enumerate() {
                    storage.store_value(
                        file_path,
                        &format!("partial_{}_{}", line, idx),
                        &replacement.local_value,
                    )?;
                }
            }
        }
    }
    
    // Apply transformations to remove local values and keep only committed values
    // This is what gets stored in Git
    let cleaned = parser.apply_decorations(content, &decorations, false);
    Ok(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_clean_inline() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = LocalStorage::new(temp_dir.path())?;
        let config = Config::default();
        
        let content = r#"let api_key = "sk-12345"; // @whiteout: "ENV_VAR""#;
        let file_path = Path::new("test.rs");
        
        let cleaned = apply(content, file_path, &storage, &config)?;
        assert!(cleaned.contains("ENV_VAR"));
        assert!(!cleaned.contains("sk-12345"));
        
        Ok(())
    }

    #[test]
    fn test_clean_block() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = LocalStorage::new(temp_dir.path())?;
        let config = Config::default();
        
        let content = r#"
// @whiteout-start
const DEBUG = true;
// @whiteout-end
const DEBUG = false;
"#;
        let file_path = Path::new("test.rs");
        
        let cleaned = apply(content, file_path, &storage, &config)?;
        assert!(cleaned.contains("false"));
        assert!(!cleaned.contains("true"));
        
        Ok(())
    }
}