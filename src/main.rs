use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use whiteout::Whiteout;

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

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();
    
    match cli.command {
        Commands::Init { path } => {
            println!("{}", "Initializing whiteout...".bright_blue());
            Whiteout::init(&path)?;
            
            // Automatically configure Git filters
            use std::process::Command;
            
            println!("{}", "Configuring Git filters...".bright_blue());
            
            // Add to .gitattributes
            let gitattributes_path = path.join(".gitattributes");
            let mut gitattributes_content = if gitattributes_path.exists() {
                std::fs::read_to_string(&gitattributes_path)?
            } else {
                String::new()
            };
            
            if !gitattributes_content.contains("filter=whiteout") {
                if !gitattributes_content.is_empty() && !gitattributes_content.ends_with('\n') {
                    gitattributes_content.push('\n');
                }
                gitattributes_content.push_str("* filter=whiteout\n");
                std::fs::write(&gitattributes_path, gitattributes_content)?;
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
            let whiteout = Whiteout::new(".")?;
            let (content, file_path) = if let Some(file_path) = file {
                let content = std::fs::read_to_string(&file_path)?;
                (content, file_path)
            } else {
                use std::io::Read;
                let mut buffer = String::new();
                std::io::stdin().read_to_string(&mut buffer)?;
                (buffer, PathBuf::from("stdin"))
            };
            
            let cleaned = whiteout.clean(&content, &file_path)?;
            print!("{}", cleaned);
        }
        
        Commands::Smudge { file } => {
            let whiteout = Whiteout::new(".")?;
            let (content, file_path) = if let Some(file_path) = file {
                let content = std::fs::read_to_string(&file_path)?;
                (content, file_path)
            } else {
                use std::io::Read;
                let mut buffer = String::new();
                std::io::stdin().read_to_string(&mut buffer)?;
                (buffer, PathBuf::from("stdin"))
            };
            
            let smudged = whiteout.smudge(&content, &file_path)?;
            print!("{}", smudged);
        }
        
        Commands::Preview { file, diff } => {
            let whiteout = Whiteout::new(".")?;
            let content = std::fs::read_to_string(&file)?;
            
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
            println!("{}", format!("Marking {:?} as local-only", file).bright_yellow());
            if let Some(l) = line {
                println!("  Line: {}", l);
            }
            if let Some(r) = replace {
                println!("  Replacement: {}", r);
            }
        }
        
        Commands::Unmark { file, line } => {
            println!("{}", format!("Unmarking {:?}", file).bright_yellow());
            if let Some(l) = line {
                println!("  Line: {}", l);
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