use super::types::Decoration;

/// Apply decorations to content
pub fn apply_decorations(
    content: &str,
    decorations: &[Decoration],
    use_local: bool,
) -> String {
    if decorations.is_empty() {
        return content.to_string();
    }

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
                        // Smudge: Check if this is a simple @whiteout or block with markers
                        let is_simple_pattern = line.contains("@whiteout") && 
                                              !line.contains("@whiteout-start") && 
                                              !line.contains("@whiteout:");
                        
                        if is_simple_pattern {
                            // Simple @whiteout: Keep marker and show local content
                            result.push(line.to_string()); // Keep @whiteout marker
                            for content_line in local_content.lines() {
                                result.push(content_line.to_string());
                            }
                            // Skip to end of the block
                            skip_until = *end_line;
                        } else {
                            // Block with markers: Keep markers and show local content
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
                        }
                    } else {
                        // Clean: Check if this is a simple @whiteout or block with markers
                        let is_simple_pattern = line.contains("@whiteout") && 
                                              !line.contains("@whiteout-start") && 
                                              !line.contains("@whiteout:");
                        
                        if is_simple_pattern {
                            // Simple @whiteout: Keep the marker, skip the local content
                            result.push(line.to_string()); // Keep @whiteout marker
                            // Skip all the local content lines
                            skip_until = *end_line;
                        } else {
                            // Block with @whiteout-start/end: Keep markers with empty content
                            result.push(line.to_string()); // Keep @whiteout-start
                            // No local content in between (it's been cleaned)
                            
                            // Add the end marker
                            if *end_line <= lines.len() {
                                result.push(lines[*end_line - 1].to_string()); // Keep @whiteout-end
                            }
                            
                            // Add the committed content that follows the block
                            if !committed_content.is_empty() {
                                for content_line in committed_content.lines() {
                                    result.push(content_line.to_string());
                                }
                            }
                            
                            // Skip to end of original block plus any following committed content
                            skip_until = *end_line + committed_content.lines().count();
                        }
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
                        result.push(format!("{} // @whiteout: {}", local_value, committed_value));
                    } else {
                        // Clean: Show committed value WITH decoration marker for smudge to work
                        result.push(format!("{} // @whiteout: {}", committed_value, committed_value));
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
                            // Clean: Preserve pattern structure with committed value for smudge to work
                            format!("[[{}||{}]]", 
                                replacement.committed_value.clone(),
                                replacement.committed_value)
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
    
    let mut output = result.join("\n");
    // Preserve trailing newline if original had one
    if content.ends_with('\n') && !output.ends_with('\n') {
        output.push('\n');
    }
    output
}