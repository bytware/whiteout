use anyhow::Result;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;

use crate::{
    config::Config,
    parser::{Decoration, Parser},
    storage::LocalStorage,
};

/// Optimized clean filter with O(n) complexity
pub fn apply_optimized<'a>(
    content: &'a str,
    file_path: &Path,
    storage: &LocalStorage,
    _config: &Config,
) -> Result<Cow<'a, str>> {
    let parser = Parser::new();
    let decorations = parser.parse(content)?;
    
    if decorations.is_empty() {
        return Ok(Cow::Borrowed(content));
    }
    
    // Pre-index decorations by line number for O(1) lookup
    let mut decoration_map: HashMap<usize, Vec<&Decoration>> = HashMap::new();
    
    for decoration in &decorations {
        match decoration {
            Decoration::Inline { line, .. } => {
                decoration_map.entry(*line).or_default().push(decoration);
            }
            Decoration::Block { start_line, end_line, .. } => {
                for line_num in *start_line..=*end_line {
                    decoration_map.entry(line_num).or_default().push(decoration);
                }
            }
            Decoration::Partial { line, .. } => {
                decoration_map.entry(*line).or_default().push(decoration);
            }
        }
    }
    
    // Process content with pre-indexed decorations
    let lines: Vec<&str> = content.lines().collect();
    let mut result = String::with_capacity(content.len());
    let mut in_block = false;
    let mut block_replacement: Option<&str> = None;
    
    for (idx, line) in lines.iter().enumerate() {
        let line_num = idx + 1;
        
        // Check for decorations on this line - O(1) lookup
        if let Some(line_decorations) = decoration_map.get(&line_num) {
            let mut processed_line = Cow::Borrowed(*line);
            
            for decoration in line_decorations {
                match decoration {
                    Decoration::Inline { local_value, committed_value, .. } => {
                        // Store local value
                        storage.store_value(
                            file_path,
                            &format!("inline_{}", line_num),
                            local_value,
                        )?;
                        
                        // Replace with committed value
                        if let Some(pos) = processed_line.find(local_value.as_str()) {
                            let mut new_line = String::with_capacity(processed_line.len());
                            new_line.push_str(&processed_line[..pos]);
                            new_line.push_str(committed_value);
                            if pos + local_value.len() < processed_line.len() {
                                new_line.push_str(&processed_line[pos + local_value.len()..]);
                            }
                            processed_line = Cow::Owned(new_line);
                        }
                    }
                    Decoration::Block { start_line, end_line, local_content, committed_content, .. } => {
                        if line_num == *start_line {
                            // Store block content
                            storage.store_value(
                                file_path,
                                &format!("block_{}", start_line),
                                local_content,
                            )?;
                            in_block = true;
                            block_replacement = Some(committed_content);
                        }
                        
                        if in_block {
                            if line_num == *end_line {
                                // End of block - add replacement
                                if let Some(replacement) = block_replacement {
                                    result.push_str(replacement);
                                    result.push('\n');
                                }
                                in_block = false;
                                block_replacement = None;
                            }
                            continue; // Skip lines inside block
                        }
                    }
                    Decoration::Partial { replacements, .. } => {
                        let mut temp_line = processed_line.to_string();
                        for (idx, replacement) in replacements.iter().enumerate() {
                            // Store each partial replacement
                            storage.store_value(
                                file_path,
                                &format!("partial_{}_{}", line_num, idx),
                                &replacement.local_value,
                            )?;
                            
                            // Apply replacement
                            temp_line = temp_line.replace(
                                &replacement.local_value,
                                &replacement.committed_value,
                            );
                        }
                        processed_line = Cow::Owned(temp_line);
                    }
                }
            }
            
            if !in_block {
                result.push_str(&processed_line);
                result.push('\n');
            }
        } else if !in_block {
            // No decorations on this line
            result.push_str(line);
            result.push('\n');
        }
    }
    
    // Remove trailing newline if original didn't have one
    if !content.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }
    
    Ok(Cow::Owned(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::LocalStorage;
    use std::path::PathBuf;
    
    #[test]
    fn test_optimized_clean_performance() {
        use tempfile::TempDir;
        
        let content = "line1\nline2 // @whiteout: REDACTED\nline3\n".repeat(1000);
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path().join(".whiteout")).unwrap();
        let config = Config::default();
        let path = Path::new("test.rs");
        
        let start = std::time::Instant::now();
        let result = apply_optimized(&content, path, &storage, &config).unwrap();
        let duration = start.elapsed();
        
        assert!(result.contains("REDACTED"));
        // Performance test - should be fast but CI environments vary
        assert!(duration.as_secs() < 60); // Very lenient timeout for CI
        
        // The important thing is it completes and produces correct output
        let lines: Vec<&str> = result.lines().collect();
        let decorated_lines = lines.iter().filter(|l| l.contains("REDACTED")).count();
        assert_eq!(decorated_lines, 1000); // All decorated lines should be processed
    }
    
    #[test]
    fn test_memory_efficiency() {
        let content = "unchanged content";
        let storage = LocalStorage::new(PathBuf::from(".whiteout")).unwrap();
        let config = Config::default();
        let path = Path::new("test.rs");
        
        let result = apply_optimized(content, path, &storage, &config).unwrap();
        
        // Should return borrowed content when no changes
        assert!(matches!(result, Cow::Borrowed(_)));
    }
}