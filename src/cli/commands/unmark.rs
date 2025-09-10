use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::path::Path;

pub fn handle(file: &Path, line: Option<String>) -> Result<()> {
    let content = std::fs::read_to_string(file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;
    
    if let Some(line_spec) = line {
        let lines: Vec<&str> = content.lines().collect();
        
        if line_spec.contains('-') {
            // Range: remove block decoration
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
            
            println!("{} Removing block decoration around lines {}-{}", 
                "→".bright_green(), start, end);
            
            // Remove @whiteout-start and @whiteout-end markers
            let mut new_lines = Vec::new();
            
            for (i, line) in lines.iter().enumerate() {
                if i > 0 && i == start - 1 && lines[i - 1].contains("@whiteout-start") {
                    continue;
                }
                if i < lines.len() - 1 && i == end && lines[i + 1].contains("@whiteout-end") {
                    continue;
                }
                if !line.contains("@whiteout-start") && !line.contains("@whiteout-end") {
                    new_lines.push(line.to_string());
                }
            }
            
            std::fs::write(file, new_lines.join("\n"))
                .with_context(|| format!("Failed to write file: {}", file.display()))?;
            
        } else {
            // Single line: remove inline decoration
            let line_num: usize = line_spec.parse()
                .context("Invalid line number")?;
            
            if line_num < 1 || line_num > lines.len() {
                bail!("Line {} out of range (file has {} lines)", 
                    line_num, lines.len());
            }
            
            println!("{} Removing decoration from line {}", 
                "→".bright_green(), line_num);
            
            let mut new_lines = Vec::new();
            for (i, line) in lines.iter().enumerate() {
                if i == line_num - 1 && line.contains("// @whiteout:") {
                    // Remove the decoration part
                    if let Some(pos) = line.find("// @whiteout:") {
                        new_lines.push(line[..pos].trim_end().to_string());
                    } else {
                        new_lines.push(line.to_string());
                    }
                } else {
                    new_lines.push(line.to_string());
                }
            }
            
            std::fs::write(file, new_lines.join("\n"))
                .with_context(|| format!("Failed to write file: {}", file.display()))?;
        }
        
        println!("{} Decoration removed successfully", "✓".bright_green());
    } else {
        // Remove all decorations from file
        println!("{} Removing all decorations from {}", 
            "→".bright_green(), file.display());
        
        let mut new_lines = Vec::new();
        let mut in_block = false;
        
        for line in content.lines() {
            if line.contains("@whiteout-start") {
                in_block = true;
                continue;
            }
            if line.contains("@whiteout-end") {
                in_block = false;
                continue;
            }
            if !in_block {
                if line.contains("// @whiteout:") {
                    if let Some(pos) = line.find("// @whiteout:") {
                        new_lines.push(line[..pos].trim_end().to_string());
                    }
                } else if !line.trim().starts_with("@whiteout") {
                    new_lines.push(line.to_string());
                }
            }
        }
        
        std::fs::write(file, new_lines.join("\n"))
            .with_context(|| format!("Failed to write file: {}", file.display()))?;
        
        println!("{} All decorations removed", "✓".bright_green());
    }
    
    Ok(())
}