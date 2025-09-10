use anyhow::Result;
use colored::Colorize;
use std::process::Command;
use walkdir::WalkDir;
use whiteout::Whiteout;

pub fn handle(verbose: bool) -> Result<()> {
    println!("{}", "Whiteout Status".bright_blue().bold());
    println!("{}", "===============".bright_blue());
    
    // Check if in a Git repository
    let git_check = Command::new("git")
        .args(&["rev-parse", "--git-dir"])
        .output()?;
    
    if !git_check.status.success() {
        println!("{} Not in a Git repository", "⚠".bright_yellow());
        return Ok(());
    }
    
    // Check if whiteout is initialized
    let _whiteout = match Whiteout::new(".") {
        Ok(w) => w,
        Err(_) => {
            println!("{} Whiteout not initialized in this project", "⚠".bright_yellow());
            println!("Run {} to initialize", "whiteout init".bright_cyan());
            return Ok(());
        }
    };
    
    println!("{} Whiteout is configured", "✓".bright_green());
    
    // Find decorated files
    let mut decorated_files = Vec::new();
    let mut total_decorations = 0;
    
    for entry in WalkDir::new(".")
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        
        // Skip hidden directories and common ignore patterns
        if path.components().any(|c| {
            c.as_os_str().to_string_lossy().starts_with('.')
                || c.as_os_str() == "target"
                || c.as_os_str() == "node_modules"
        }) {
            continue;
        }
        
        if let Ok(content) = std::fs::read_to_string(path) {
            let mut decorations = 0;
            
            // Count decorations
            decorations += content.matches("@whiteout:").count();
            decorations += content.matches("@whiteout-start").count();
            decorations += content.matches("@whiteout-partial").count();
            decorations += content.lines()
                .filter(|l| l.trim() == "@whiteout")
                .count();
            
            if decorations > 0 {
                decorated_files.push((path.to_path_buf(), decorations));
                total_decorations += decorations;
            }
        }
    }
    
    if decorated_files.is_empty() {
        println!("\n{} No decorated files found", "ℹ".bright_blue());
    } else {
        println!("\n{}", format!("Found {} decorated files with {} total decorations:",
            decorated_files.len(), total_decorations).bright_green());
        
        for (file, count) in &decorated_files {
            if verbose {
                println!("  {} {} ({} decorations)", 
                    "•".bright_cyan(), 
                    file.display(), 
                    count);
                
                // Show decoration details
                if let Ok(content) = std::fs::read_to_string(file) {
                    for (line_num, line) in content.lines().enumerate() {
                        if line.contains("@whiteout") {
                            println!("      {} Line {}: {}", 
                                "→".bright_black(),
                                line_num + 1,
                                line.trim().bright_black());
                        }
                    }
                }
            } else {
                println!("  {} {}", "•".bright_cyan(), file.display());
            }
        }
        
        if !verbose {
            println!("\n{}", "Tip: Use --verbose for detailed information".bright_cyan());
        }
    }
    
    Ok(())
}