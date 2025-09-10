mod cli;

use anyhow::Result;
use clap::Parser;
use cli::commands;

fn main() {
    // Setup tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = cli::Cli::parse();
    
    // Run the command and handle errors gracefully
    if let Err(err) = run_command(cli) {
        commands::display_error(&err);
        std::process::exit(1);
    }
}

fn run_command(cli: cli::Cli) -> Result<()> {
    use cli::Commands;
    
    match cli.command {
        Commands::Init { path } => commands::init::handle(&path),
        Commands::Clean { file } => commands::clean::handle(file),
        Commands::Smudge { file } => commands::smudge::handle(file),
        Commands::Preview { file, diff } => commands::preview::handle(&file, diff),
        Commands::Check { files, fix } => commands::check::handle(files, fix),
        Commands::Mark { file, line, replace } => commands::mark::handle(&file, line, replace),
        Commands::Unmark { file, line } => commands::unmark::handle(&file, line),
        Commands::Status { verbose } => commands::status::handle(verbose),
        Commands::Config { action } => commands::config::handle(action),
        Commands::Sync { branch } => commands::sync::handle(branch),
    }
}