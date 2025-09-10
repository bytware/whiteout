use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;
use std::process::Command;
use whiteout::Whiteout;

pub fn handle(path: &Path) -> Result<()> {
    println!("{}", "Initializing Whiteout in your project...".bright_blue().bold());
    println!();
    
    // Create .whiteout directory
    println!("{}", "Setting up local storage:".bright_blue());
    Whiteout::init(path)
        .context("Failed to initialize Whiteout in the specified directory")?;
    println!("  {} Created .whiteout/ directory for local values", "✓".bright_green());
    
    // Automatically configure Git filters
    println!("\n{}", "Configuring Git integration:".bright_blue());
    
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
    println!("  {} Set clean filter (removes secrets before commit)", "✓".bright_green());
    
    Command::new("git")
        .args(&["config", "filter.whiteout.smudge", "whiteout smudge"])
        .current_dir(path)
        .output()?;
    println!("  {} Set smudge filter (restores secrets after checkout)", "✓".bright_green());
    
    Command::new("git")
        .args(&["config", "filter.whiteout.required", "true"])
        .current_dir(path)
        .output()?;
    println!("  {} Enabled filter requirement (prevents accidental commits)", "✓".bright_green());
    
    println!("\n{}", "✨ Whiteout initialized successfully!".bright_green().bold());
    println!("\n{}", "What this means:".bright_blue().bold());
    println!("  • Your secrets will stay local and never reach Git");
    println!("  • Safe alternatives will be committed instead");
    println!("  • Local values are stored in .whiteout/ (gitignored)");
    
    println!("\n{}", "Quick start:".bright_blue().bold());
    println!("  1. Add decorations to your code:");
    println!("     {}", "let key = \"process.env.API_KEY\"; // @whiteout: \"sk-12345\"".bright_yellow());
    println!("  2. Commit normally - secrets stay local!");
    
    Ok(())
}