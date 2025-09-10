use anyhow::Result;
use colored::Colorize;
use regex::Regex;
use std::path::PathBuf;
use std::process::Command;

pub fn handle(files: Vec<PathBuf>, fix: bool) -> Result<()> {
    println!("{}", "Checking for potential secrets...".bright_blue());
    
    // Simple pattern matching for potential secrets
    let patterns = vec![
        (r"(?i)(api[_-]?key|apikey)", "API Key"),
        (r"(?i)(secret|password|passwd|pwd)", "Secret/Password"),
        (r"(?i)(token|bearer)", "Token"),
        (r"(?i)sk-[a-zA-Z0-9]{32,}", "OpenAI API Key"),
        (r"(?i)ghp_[a-zA-Z0-9]{36}", "GitHub Token"),
        (r"https?://[^/]*:[^@]*@", "URL with credentials"),
    ];
    
    let files_to_check = if files.is_empty() {
        // Get all tracked files
        let output = Command::new("git")
            .args(&["ls-files"])
            .output()?;
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(PathBuf::from)
            .collect()
    } else {
        files
    };
    
    let mut found_issues = false;
    for file_path in files_to_check {
        if let Ok(content) = std::fs::read_to_string(&file_path) {
            for (pattern_str, name) in &patterns {
                let regex = Regex::new(pattern_str)?;
                for (line_num, line) in content.lines().enumerate() {
                    // Skip if already decorated
                    if line.contains("@whiteout") {
                        continue;
                    }
                    
                    if regex.is_match(line) {
                        found_issues = true;
                        println!(
                            "{} {} in {}:{} - {}",
                            "⚠".bright_yellow(),
                            name,
                            file_path.display(),
                            line_num + 1,
                            line.trim().bright_red()
                        );
                        
                        if fix {
                            // TODO: Implement auto-fix logic
                            println!("  {} Auto-fix not yet implemented", "→".bright_cyan());
                        }
                    }
                }
            }
        }
    }
    
    if !found_issues {
        println!("{} No potential secrets found!", "✓".bright_green());
    } else if !fix {
        println!("\n{}", "Tip: Use --fix to automatically add decorations".bright_cyan());
    }
    
    Ok(())
}