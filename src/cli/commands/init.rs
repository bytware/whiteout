use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;
use std::process::Command;
use whiteout::Whiteout;

pub fn handle(path: &Path) -> Result<()> {
    println!("{}", "Initializing whiteout...".bright_blue());
    Whiteout::init(path)
        .context("Failed to initialize Whiteout in the specified directory")?;
    
    // Automatically configure Git filters
    println!("{}", "Configuring Git filters...".bright_blue());
    
    // Add to .gitattributes
    let gitattributes_path = path.join(".gitattributes");
    let mut gitattributes_content = if gitattributes_path.exists() {
        std::fs::read_to_string(&gitattributes_path)
            .context("Failed to read .gitattributes file")?
    } else {
        String::new()
    };
    
    if !gitattributes_content.contains("filter=whiteout") {
        if !gitattributes_content.is_empty() && !gitattributes_content.ends_with('\n') {
            gitattributes_content.push('\n');
        }
        gitattributes_content.push_str("* filter=whiteout\n");
        std::fs::write(&gitattributes_path, gitattributes_content)
            .context("Failed to write .gitattributes file")?;
        println!("  {} Added filter to .gitattributes", "✓".bright_green());
    }
    
    // Configure Git
    Command::new("git")
        .args(&["config", "filter.whiteout.clean", "whiteout clean"])
        .current_dir(path)
        .output()?;
    
    Command::new("git")
        .args(&["config", "filter.whiteout.smudge", "whiteout smudge"])
        .current_dir(path)
        .output()?;
    
    Command::new("git")
        .args(&["config", "filter.whiteout.required", "true"])
        .current_dir(path)
        .output()?;
    
    println!("  {} Configured Git filters", "✓".bright_green());
    println!("\n{}", "✓ Whiteout initialized and configured successfully!".bright_green().bold());
    println!("\n{}", "Quick start:".bright_blue().bold());
    println!("  1. Add decorations to your code:");
    println!("     {}", "let key = \"secret\"; // @whiteout: \"REDACTED\"".bright_yellow());
    println!("  2. Commit normally - secrets stay local!");
    
    Ok(())
}