use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::path::Path;

pub fn handle(file: &Path, line: Option<String>, replace: Option<String>) -> Result<()> {
    let content = std::fs::read_to_string(file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;
    
    if let Some(line_spec) = line {
        let replacement = replace.unwrap_or_else(|| "REDACTED".to_string());
        
        // Parse line number or range
        let lines: Vec<&str> = content.lines().collect();
        
        if line_spec.contains('-') {
            // Range: e.g., "10-15"
            let parts: Vec<&str> = line_spec.split('-').collect();
            if parts.len() != 2 {
                bail!("Invalid line range format. Use: start-end");
            }
            
            let start: usize = parts[0].parse()
                .context("Invalid start line number")?;
            let end: usize = parts[1].parse()
                .context("Invalid end line number")?;
            
            if start < 1 || end > lines.len() || start > end {
                bail!("Invalid line range: {} (file has {} lines)", line_spec, lines.len());
            }
            
            println!("{} Marking lines {}-{} as local-only in {}", 
                "→".bright_green(), start, end, file.display());
            
            // Add block decoration
            let mut new_lines = Vec::new();
            for (i, line) in lines.iter().enumerate() {
                if i == start - 1 {
                    new_lines.push(format!("// @whiteout-start"));
                }
                new_lines.push(line.to_string());
                if i == end - 1 {
                    new_lines.push(format!("// @whiteout-end"));
                    // Add replacement as comment
                    for repl_line in replacement.lines() {
                        new_lines.push(format!("// {}", repl_line));
                    }
                }
            }
            
            std::fs::write(file, new_lines.join("\n"))
                .with_context(|| format!("Failed to write file: {}", file.display()))?;
            
        } else {
            // Single line
            let line_num: usize = line_spec.parse()
                .context("Invalid line number")?;
            
            if line_num < 1 || line_num > lines.len() {
                bail!("Line {} out of range (file has {} lines)", 
                    line_num, lines.len());
            }
            
            println!("{} Marking line {} as local-only in {}", 
                "→".bright_green(), line_num, file.display());
            
            // Add inline decoration
            let mut new_lines = Vec::new();
            for (i, line) in lines.iter().enumerate() {
                if i == line_num - 1 {
                    // Check if it already has a decoration
                    if line.contains("@whiteout") {
                        println!("{} Line already has decoration", "⚠".bright_yellow());
                        new_lines.push(line.to_string());
                    } else {
                        new_lines.push(format!("{} // @whiteout: {}", 
                            line, replacement));
                    }
                } else {
                    new_lines.push(line.to_string());
                }
            }
            
            std::fs::write(file, new_lines.join("\n"))
                .with_context(|| format!("Failed to write file: {}", file.display()))?;
        }
        
        println!("{} File updated successfully", "✓".bright_green());
    } else {
        println!("{}", "Interactive mode:".bright_blue().bold());
        println!("  {} Line marking: {}", "•".bright_cyan(), 
            "whiteout mark file.rs -l 10 -r 'REDACTED'".bright_yellow());
        println!("  {} Block marking: {}", "•".bright_cyan(), 
            "whiteout mark file.rs -l 10-20 -r 'REDACTED'".bright_yellow());
    }
    
    Ok(())
}