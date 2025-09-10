pub mod commands;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "whiteout")]
#[command(about = "Local-only code decoration tool for Git", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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
pub enum ConfigAction {
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