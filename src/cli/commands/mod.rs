pub mod init;
pub mod clean;
pub mod smudge;
pub mod preview;
pub mod check;
pub mod mark;
pub mod unmark;
pub mod status;
pub mod config;
pub mod sync;

use colored::Colorize;

/// Display an error message with proper formatting
pub fn display_error(err: &anyhow::Error) {
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