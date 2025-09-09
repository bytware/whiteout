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
    let parser = Parser::new();
    let mut decorations = parser.parse(content)?;
    
    if decorations.is_empty() {
        return Ok(content.to_string());
    }
    
    for decoration in &mut decorations {
        match decoration {
            Decoration::Inline { line, local_value, .. } => {
                if let Ok(stored_value) = storage.get_value(
                    file_path,
                    &format!("inline_{}", line),
                ) {
                    *local_value = stored_value;
                }
            }
            Decoration::Block { start_line, local_content, .. } => {
                if let Ok(stored_value) = storage.get_value(
                    file_path,
                    &format!("block_{}", start_line),
                ) {
                    *local_content = stored_value;
                }
            }
            Decoration::Partial { line, replacements } => {
                for (idx, replacement) in replacements.iter_mut().enumerate() {
                    if let Ok(stored_value) = storage.get_value(
                        file_path,
                        &format!("partial_{}_{}", line, idx),
                    ) {
                        replacement.local_value = stored_value;
                    }
                }
            }
        }
    }
    
    let smudged = parser.apply_decorations(content, &decorations, true);
    
    Ok(smudged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_smudge_inline() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = LocalStorage::new(temp_dir.path())?;
        let config = Config::default();
        let file_path = Path::new("test.rs");
        
        storage.store_value(file_path, "inline_1", "let api_key = \"sk-12345\";")?;
        
        let content = r#"let api_key = "ENV_VAR"; // @whiteout: "ENV_VAR""#;
        
        let smudged = apply(content, file_path, &storage, &config)?;
        assert!(smudged.contains("sk-12345"));
        assert!(!smudged.contains("ENV_VAR"));
        
        Ok(())
    }

    #[test]
    fn test_smudge_block() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = LocalStorage::new(temp_dir.path())?;
        let config = Config::default();
        let file_path = Path::new("test.rs");
        
        storage.store_value(file_path, "block_2", "const DEBUG = true;")?;
        
        let content = r#"
// @whiteout-start
const DEBUG = false;
// @whiteout-end
const DEBUG = false;
"#;
        
        let smudged = apply(content, file_path, &storage, &config)?;
        assert!(smudged.contains("true"));
        
        Ok(())
    }
}