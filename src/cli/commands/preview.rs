use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;
use whiteout::Whiteout;

pub fn handle(file: &Path, diff: bool) -> Result<()> {
    let whiteout = Whiteout::new(".")
        .context("Failed to load Whiteout configuration")?;
    
    let content = std::fs::read_to_string(file)
        .with_context(|| format!("Failed to read file: {}", file.display()))?;
    
    println!("{}", "Whiteout Preview".bright_blue().bold());
    println!("{}", "================".bright_blue());
    println!("File: {}\n", file.display());
    
    let cleaned = whiteout.clean(&content, file)?;
    
    if diff {
        println!("{}", "LOCAL VERSION (Your Working Directory):".bright_green().bold());
        println!("{}", "----------------------------------------".bright_green());
        println!("{}", content);
        println!();
        println!("{}", "COMMITTED VERSION (What Git Will Store):".bright_yellow().bold());
        println!("{}", "-----------------------------------------".bright_yellow());
        println!("{}", cleaned);
    } else {
        println!("{}", "What will be committed:".bright_yellow().bold());
        println!("{}", cleaned);
        println!();
        println!("Use {} to see side-by-side comparison", "--diff".bright_cyan());
    }
    
    Ok(())
}