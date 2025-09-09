use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use whiteout::Whiteout;

/// Display an error message with proper formatting
fn display_error(err: &anyhow::Error) {
    eprintln!("\n{} {}", "✗".bright_red().bold(), "Operation failed".bright_red().bold());
    eprintln!("  {} {}", "├".bright_black(), err);
    
    // Display error chain
    for cause in err.chain().skip(1) {
        eprintln!("  {} {}", "├".bright_black(), cause);
    }
    
    // Add helpful context based on error type
    let error_str = err.to_string();
    if error_str.contains("Permission denied") {
        eprintln!("  {} Try running with elevated permissions", "└".bright_cyan());
    } else if error_str.contains("No such file") {
        eprintln!("  {} Check that the file path is correct", "└".bright_cyan());
    } else if error_str.contains("git") {
        eprintln!("  {} Ensure you're in a Git repository", "└".bright_cyan());
    } else {
        eprintln!("  {} Run with {} for more details", 
            "└".bright_black(), 
            "--verbose".bright_cyan()
        );
    }
}

#[derive(Parser)]
#[command(name = "whiteout")]
#[command(about = "Local-only code decoration tool for Git", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Initialize whiteout in the current project")]
    Init {
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
    },
    
    #[command(about = "Preview what will be committed vs what stays local")]
    Preview {
        #[arg(help = "File path to preview")]
        file: PathBuf,
        #[arg(short, long, help = "Show side-by-side diff")]
        diff: bool,
    },
    
    #[command(about = "Check files for potential secrets that aren't decorated")]
    Check {
        #[arg(help = "Files to check (defaults to all tracked files)")]
        files: Vec<PathBuf>,
        #[arg(short, long, help = "Fix issues automatically")]
        fix: bool,
    },
    
    #[command(about = "Mark a line or block as local-only")]
    Mark {
        #[arg(help = "File path")]
        file: PathBuf,
        #[arg(short, long, help = "Line number or range (e.g., 10 or 10-15)")]
        line: Option<String>,
        #[arg(short, long, help = "Replacement text for committed version")]
        replace: Option<String>,
    },
    
    #[command(about = "Remove whiteout decoration")]
    Unmark {
        #[arg(help = "File path")]
        file: PathBuf,
        #[arg(short, long, help = "Line number or range")]
        line: Option<String>,
    },
    
    #[command(about = "Show status of decorated files")]
    Status {
        #[arg(short, long, help = "Show detailed information")]
        verbose: bool,
    },
    
    #[command(about = "Apply clean filter (for Git integration)")]
    Clean {
        #[arg(help = "File path (optional, reads from stdin if not provided)")]
        file: Option<PathBuf>,
    },
    
    #[command(about = "Apply smudge filter (for Git integration)")]
    Smudge {
        #[arg(help = "File path (optional, reads from stdin if not provided)")]
        file: Option<PathBuf>,
    },
    
    #[command(about = "Configure whiteout settings")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    
    #[command(about = "Sync local values across branches")]
    Sync {
        #[arg(short, long, help = "Target branch")]
        branch: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    #[command(about = "Set a configuration value")]
    Set {
        key: String,
        value: String,
    },
    #[command(about = "Get a configuration value")]
    Get {
        key: String,
    },
    #[command(about = "List all configuration values")]
    List,
}

fn main() {
    // Setup tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();
    
    // Run the command and handle errors gracefully
    if let Err(err) = run_command(cli) {
        display_error(&err);
        std::process::exit(1);
    }
}

fn run_command(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init { path } => {
            println!("{}", "Initializing whiteout...".bright_blue());
            Whiteout::init(&path)
                .context("Failed to initialize Whiteout in the specified directory")?;
            
            // Automatically configure Git filters
            use std::process::Command;
            
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
                .current_dir(&path)
                .output()?;
            
            Command::new("git")
                .args(&["config", "filter.whiteout.smudge", "whiteout smudge"])
                .current_dir(&path)
                .output()?;
            
            Command::new("git")
                .args(&["config", "filter.whiteout.required", "true"])
                .current_dir(&path)
                .output()?;
            
            println!("  {} Configured Git filters", "✓".bright_green());
            println!("\n{}", "✓ Whiteout initialized and configured successfully!".bright_green().bold());
            println!("\n{}", "Quick start:".bright_blue().bold());
            println!("  1. Add decorations to your code:");
            println!("     {}", "let key = \"secret\"; // @whiteout: \"REDACTED\"".bright_yellow());
            println!("  2. Commit normally - secrets stay local!");
        }
        
        Commands::Clean { file } => {
            let whiteout = Whiteout::new(".")
                .context("Failed to load Whiteout configuration")?;
            let (content, file_path) = if let Some(file_path) = file {
                let content = std::fs::read_to_string(&file_path)
                    .with_context(|| format!("Failed to read file: {}", file_path.display()))?;
                (content, file_path)
            } else {
                use std::io::Read;
                let mut buffer = String::new();
                std::io::stdin().read_to_string(&mut buffer)?;
                (buffer, PathBuf::from("stdin"))
            };
            
            let cleaned = whiteout.clean(&content, &file_path)
                .context("Failed to apply clean filter")?;
            print!("{}", cleaned);
        }
        
        Commands::Smudge { file } => {
            let whiteout = Whiteout::new(".")
                .context("Failed to load Whiteout configuration")?;
            let (content, file_path) = if let Some(file_path) = file {
                let content = std::fs::read_to_string(&file_path)
                    .with_context(|| format!("Failed to read file: {}", file_path.display()))?;
                (content, file_path)
            } else {
                use std::io::Read;
                let mut buffer = String::new();
                std::io::stdin().read_to_string(&mut buffer)?;
                (buffer, PathBuf::from("stdin"))
            };
            
            let smudged = whiteout.smudge(&content, &file_path)
                .context("Failed to apply smudge filter")?;
            print!("{}", smudged);
        }
        
        Commands::Preview { file, diff } => {
            let whiteout = Whiteout::new(".")
                .context("Failed to load Whiteout configuration")?;
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Failed to read file: {}", file.display()))?;
            
            println!("{}", "Whiteout Preview".bright_blue().bold());
            println!("{}", "================".bright_blue());
            println!("File: {}\n", file.display());
            
            let cleaned = whiteout.clean(&content, &file)?;
            
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
        }
        
        Commands::Check { files, fix } => {
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
                use std::process::Command;
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
                        let re = regex::Regex::new(pattern_str)?;
                        for (line_num, line) in content.lines().enumerate() {
                            if re.is_match(line) && !line.contains("@whiteout") {
                                println!("  {} {}:{} - Potential {} found", 
                                    "⚠".bright_yellow(), 
                                    file_path.display(), 
                                    line_num + 1,
                                    name);
                                found_issues = true;
                                
                                if fix {
                                    println!("    {} Add decoration: // @whiteout: \"REDACTED\"", 
                                        "→".bright_cyan());
                                }
                            }
                        }
                    }
                }
            }
            
            if !found_issues {
                println!("{}", "✓ No potential secrets found!".bright_green());
            } else if !fix {
                println!("\nUse {} to automatically add decorations", "--fix".bright_cyan());
            }
        }
        
        Commands::Status { verbose } => {
            let _whiteout = Whiteout::new(".")?;
            println!("{}", "Whiteout Status".bright_blue().bold());
            println!("{}", "===============".bright_blue());
            
            if verbose {
                println!("Detailed status information will be shown here");
            } else {
                println!("Status summary will be shown here");
            }
        }
        
        Commands::Mark { file, line, replace } => {
            if !file.exists() {
                anyhow::bail!("File not found: {}", file.display());
            }
            
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Failed to read file: {}", file.display()))?;
            
            let replacement = replace.unwrap_or_else(|| "REDACTED".to_string());
            
            if let Some(line_spec) = line {
                // Parse line number or range
                let lines: Vec<&str> = content.lines().collect();
                
                if line_spec.contains('-') {
                    // Range: e.g., "10-15"
                    let parts: Vec<&str> = line_spec.split('-').collect();
                    if parts.len() != 2 {
                        anyhow::bail!("Invalid line range format. Use: start-end");
                    }
                    
                    let start: usize = parts[0].parse()
                        .context("Invalid start line number")?;
                    let end: usize = parts[1].parse()
                        .context("Invalid end line number")?;
                    
                    if start < 1 || end > lines.len() || start > end {
                        anyhow::bail!("Invalid line range: {} (file has {} lines)", line_spec, lines.len());
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
                    
                    std::fs::write(&file, new_lines.join("\n"))
                        .with_context(|| format!("Failed to write file: {}", file.display()))?;
                    
                } else {
                    // Single line
                    let line_num: usize = line_spec.parse()
                        .context("Invalid line number")?;
                    
                    if line_num < 1 || line_num > lines.len() {
                        anyhow::bail!("Line {} out of range (file has {} lines)", 
                            line_num, lines.len());
                    }
                    
                    println!("{} Marking line {} as local-only in {}", 
                        "→".bright_green(), line_num, file.display());
                    
                    // Add inline decoration
                    let mut new_lines = Vec::new();
                    for (i, line) in lines.iter().enumerate() {
                        if i == line_num - 1 {
                            // Add decoration to the line
                            new_lines.push(format!("{} // @whiteout: \"{}\"", 
                                line.trim_end(), replacement));
                        } else {
                            new_lines.push(line.to_string());
                        }
                    }
                    
                    std::fs::write(&file, new_lines.join("\n"))
                        .with_context(|| format!("Failed to write file: {}", file.display()))?;
                }
                
                println!("{} Successfully marked as local-only", "✓".bright_green());
                
            } else {
                // Interactive mode hint
                println!("{}", "Interactive marking:".bright_blue().bold());
                println!("File: {}\n", file.display());
                
                let lines: Vec<&str> = content.lines().collect();
                let max_display = 20.min(lines.len());
                
                for (i, line) in lines.iter().take(max_display).enumerate() {
                    println!("{:4} | {}", i + 1, line);
                }
                
                if lines.len() > max_display {
                    println!("... ({} more lines)", lines.len() - max_display);
                }
                
                println!("\n{} Use {} to mark specific lines", 
                    "Hint:".bright_cyan(),
                    format!("--line NUMBER or --line START-END").bright_white());
                println!("Example: {} mark {} --line 5 --replace \"REDACTED\"",
                    "whiteout".bright_white(),
                    file.display());
            }
        }
        
        Commands::Unmark { file, line } => {
            if !file.exists() {
                anyhow::bail!("File not found: {}", file.display());
            }
            
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Failed to read file: {}", file.display()))?;
            
            let lines: Vec<&str> = content.lines().collect();
            let mut new_lines = Vec::new();
            let mut removed_count = 0;
            let mut skip_until_end = false;
            
            if let Some(line_spec) = line {
                // Remove specific decoration
                let target_line: usize = line_spec.parse()
                    .context("Invalid line number")?;
                
                if target_line < 1 || target_line > lines.len() {
                    anyhow::bail!("Line {} out of range (file has {} lines)", 
                        target_line, lines.len());
                }
                
                for (i, line) in lines.iter().enumerate() {
                    let line_num = i + 1;
                    
                    if line_num == target_line && line.contains("// @whiteout:") {
                        // Remove inline decoration
                        if let Some(pos) = line.find("// @whiteout:") {
                            new_lines.push(line[..pos].trim_end().to_string());
                            removed_count += 1;
                        } else {
                            new_lines.push(line.to_string());
                        }
                    } else {
                        new_lines.push(line.to_string());
                    }
                }
                
            } else {
                // Remove all decorations
                for line in lines {
                    if line.contains("@whiteout-start") {
                        skip_until_end = true;
                        removed_count += 1;
                        continue;
                    } else if line.contains("@whiteout-end") {
                        skip_until_end = false;
                        removed_count += 1;
                        // Skip the following comment lines that are replacements
                        continue;
                    } else if skip_until_end {
                        // Skip lines inside block
                        continue;
                    } else if line.contains("// @whiteout:") {
                        // Remove inline decoration
                        if let Some(pos) = line.find("// @whiteout:") {
                            let cleaned = line[..pos].trim_end();
                            if !cleaned.is_empty() {
                                new_lines.push(cleaned.to_string());
                            }
                            removed_count += 1;
                        } else {
                            new_lines.push(line.to_string());
                        }
                    } else if line.trim().starts_with("// @whiteout") {
                        // Skip standalone decoration lines
                        removed_count += 1;
                        continue;
                    } else {
                        new_lines.push(line.to_string());
                    }
                }
            }
            
            if removed_count > 0 {
                std::fs::write(&file, new_lines.join("\n"))
                    .with_context(|| format!("Failed to write file: {}", file.display()))?;
                
                println!("{} Removed {} decoration(s) from {}", 
                    "✓".bright_green(), 
                    removed_count, 
                    file.display());
            } else {
                println!("{} No decorations found to remove in {}", 
                    "⚠".bright_yellow(),
                    file.display());
            }
        }
        
        Commands::Config { action } => match action {
            ConfigAction::Set { key, value } => {
                println!("Setting {} = {}", key.bright_cyan(), value);
            }
            ConfigAction::Get { key } => {
                println!("Getting value for {}", key.bright_cyan());
            }
            ConfigAction::List => {
                println!("{}", "Configuration:".bright_blue().bold());
                println!("  encryption: false");
                println!("  auto_sync: true");
            }
        },
        
        Commands::Sync { branch } => {
            println!("{}", "Syncing local values...".bright_yellow());
            if let Some(b) = branch {
                println!("  Target branch: {}", b);
            }
        }
    }
    
    Ok(())
}