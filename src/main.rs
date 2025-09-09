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
            println!("{}", "✓ Whiteout initialized successfully!".bright_green());
            println!("\nNext steps:");
            println!("  1. Add Git filter configuration to your .gitattributes:");
            println!("     {}", "* filter=whiteout".bright_yellow());
            println!("  2. Configure Git filters:");
            println!("     {}", "git config filter.whiteout.clean 'whiteout clean'".bright_yellow());
            println!("     {}", "git config filter.whiteout.smudge 'whiteout smudge'".bright_yellow());
            println!("     {}", "git config filter.whiteout.required true".bright_yellow());
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