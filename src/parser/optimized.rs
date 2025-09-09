use std::collections::HashMap;
use std::borrow::Cow;
use super::{Decoration, PartialReplacement};

/// Optimized decoration application with O(n) complexity instead of O(n*m)
/// Pre-indexes decorations by line number for constant-time lookup
pub fn apply_decorations_optimized<'a>(
    content: &'a str,
    decorations: &[Decoration],
    use_local: bool,
) -> String {
    // Early return for no decorations
    if decorations.is_empty() {
        return content.to_string();
    }

    let lines: Vec<&str> = content.lines().collect();
    
    // Pre-allocate result vector with exact capacity
    let mut result = Vec::with_capacity(lines.len());
    
    // Index decorations by line number for O(1) lookup
    let mut inline_map: HashMap<usize, &Decoration> = HashMap::new();
    let mut block_map: HashMap<usize, &Decoration> = HashMap::new();
    let mut partial_map: HashMap<usize, &Decoration> = HashMap::new();
    
    for decoration in decorations {
        match decoration {
            Decoration::Inline { line, .. } => {
                inline_map.insert(*line, decoration);
            }
            Decoration::Block { start_line, .. } => {
                block_map.insert(*start_line, decoration);
            }
            Decoration::Partial { line, .. } => {
                partial_map.insert(*line, decoration);
            }
        }
    }
    
    let mut skip_until = 0;
    
    for (idx, line) in lines.iter().enumerate() {
        let line_num = idx + 1;
        
        // Skip lines that are part of a processed block
        if line_num <= skip_until {
            continue;
        }
        
        // Check for block decoration (highest priority)
        if let Some(decoration) = block_map.get(&line_num) {
            if let Decoration::Block { 
                start_line: _, 
                end_line, 
                local_content, 
                committed_content 
            } = decoration {
                if use_local {
                    // Smudge: Keep markers and show local content
                    result.push(line.to_string()); // Keep @whiteout-start
                    for content_line in local_content.lines() {
                        result.push(content_line.to_string());
                    }
                    // Add end marker
                    if *end_line <= lines.len() {
                        result.push(lines[*end_line - 1].to_string());
                    }
                    // Skip the committed content
                    skip_until = *end_line + committed_content.lines().count();
                } else {
                    // Clean: Show only committed content
                    if !committed_content.is_empty() {
                        for content_line in committed_content.lines() {
                            result.push(content_line.to_string());
                        }
                    }
                    skip_until = *end_line + committed_content.lines().count();
                }
                continue;
            }
        }
        
        // Check for inline decoration
        if let Some(decoration) = inline_map.get(&line_num) {
            if let Decoration::Inline { 
                line: _, 
                local_value, 
                committed_value 
            } = decoration {
                if use_local {
                    result.push(format!("{} // @whiteout: \"{}\"", 
                        local_value, committed_value));
                } else {
                    result.push(committed_value.to_string());
                }
                continue;
            }
        }
        
        // Check for partial replacements
        if let Some(decoration) = partial_map.get(&line_num) {
            if let Decoration::Partial { line: _, replacements } = decoration {
                let mut processed_line = line.to_string();
                
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
        }
        
        // No decoration for this line - add as-is
        result.push(line.to_string());
    }
    
    result.join("\n")
}

/// Memory-efficient version using Cow to avoid unnecessary allocations
pub fn apply_decorations_zero_copy(
    content: &str,
    decorations: &[Decoration],
    use_local: bool,
) -> String {
    if decorations.is_empty() {
        return content.to_string();
    }

    let lines: Vec<&str> = content.lines().collect();
    let mut result: Vec<Cow<'_, str>> = Vec::with_capacity(lines.len());
    
    // Index decorations
    let mut decoration_map: HashMap<usize, Vec<&Decoration>> = HashMap::new();
    for decoration in decorations {
        let line_num = match decoration {
            Decoration::Inline { line, .. } => *line,
            Decoration::Block { start_line, .. } => *start_line,
            Decoration::Partial { line, .. } => *line,
        };
        decoration_map.entry(line_num).or_default().push(decoration);
    }
    
    let mut skip_until = 0;
    
    for (idx, line) in lines.iter().enumerate() {
        let line_num = idx + 1;
        
        if line_num <= skip_until {
            continue;
        }
        
        if let Some(line_decorations) = decoration_map.get(&line_num) {
            let mut processed = false;
            
            for decoration in line_decorations {
                match decoration {
                    Decoration::Block { end_line, local_content, committed_content, .. } => {
                        if use_local {
                            result.push(Cow::Borrowed(*line));
                            for content_line in local_content.lines() {
                                result.push(Cow::Owned(content_line.to_string()));
                            }
                            if *end_line <= lines.len() {
                                result.push(Cow::Borrowed(lines[*end_line - 1]));
                            }
                            skip_until = *end_line + committed_content.lines().count();
                        } else {
                            for content_line in committed_content.lines() {
                                result.push(Cow::Owned(content_line.to_string()));
                            }
                            skip_until = *end_line + committed_content.lines().count();
                        }
                        processed = true;
                        break;
                    }
                    Decoration::Inline { local_value, committed_value, .. } => {
                        if use_local {
                            result.push(Cow::Owned(
                                format!("{} // @whiteout: \"{}\"", local_value, committed_value)
                            ));
                        } else {
                            result.push(Cow::Owned(committed_value.to_string()));
                        }
                        processed = true;
                        break;
                    }
                    Decoration::Partial { replacements, .. } => {
                        let mut processed_line = (*line).to_string();
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
                        result.push(Cow::Owned(processed_line));
                        processed = true;
                        break;
                    }
                }
            }
            
            if processed {
                continue;
            }
        }
        
        // No decoration - use borrowed reference
        result.push(Cow::Borrowed(*line));
    }
    
    // Convert Cow strings to owned string
    result.into_iter()
        .map(|cow| cow.into_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_performance() {
        let content = "line1\nline2\nline3\nline4\nline5";
        let decorations = vec![
            Decoration::Inline {
                line: 2,
                local_value: "secret".to_string(),
                committed_value: "REDACTED".to_string(),
            },
            Decoration::Inline {
                line: 4,
                local_value: "password".to_string(),
                committed_value: "HIDDEN".to_string(),
            },
        ];
        
        let result = apply_decorations_optimized(content, &decorations, false);
        assert!(result.contains("REDACTED"));
        assert!(result.contains("HIDDEN"));
        assert!(!result.contains("secret"));
        assert!(!result.contains("password"));
    }

    #[test]
    fn test_zero_copy_efficiency() {
        let content = "unchanged1\nunchanged2\nchanged\nunchanged3";
        let decorations = vec![
            Decoration::Inline {
                line: 3,
                local_value: "secret".to_string(),
                committed_value: "REDACTED".to_string(),
            },
        ];
        
        let result = apply_decorations_zero_copy(content, &decorations, false);
        assert!(result.contains("unchanged1"));
        assert!(result.contains("REDACTED"));
        assert!(!result.contains("secret"));
    }
}