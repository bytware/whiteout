use anyhow::{Context, Result};
use std::path::PathBuf;
use std::io::Read;
use whiteout::Whiteout;

pub fn handle(file: Option<PathBuf>) -> Result<()> {
    let whiteout = Whiteout::new(".")
        .context("Failed to load Whiteout configuration")?;
    
    let (content, file_path) = if let Some(file_path) = file {
        let content = std::fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;
        (content, file_path)
    } else {
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        (buffer, PathBuf::from("stdin"))
    };
    
    let smudged = whiteout.smudge(&content, &file_path)
        .context("Failed to apply smudge filter")?;
    print!("{}", smudged);
    
    Ok(())
}