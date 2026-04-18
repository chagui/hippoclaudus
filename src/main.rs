use anyhow::Result;
use clap::{Parser, Subcommand};
use std::process;

use hippoclaudus::config::Config;

mod commands;
mod helpers;
mod prompt;
mod sync;

#[derive(Parser)]
#[command(name = "hpc")]
#[command(
    about = "Hippoclaudus — Claude Code companion for session monitoring, knowledge sync, and vault search"
)]
struct Cli {
    /// Enable verbose logging (RUST_LOG=debug)
    #[arg(long, short, global = true)]
    verbose: bool,

    /// Suppress all log output (RUST_LOG=off)
    #[arg(long, short, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract conversation text from session JSONL files
    Extract {
        #[arg(long, default_value = "7")]
        days: u32,
        #[arg(long, default_value = "500")]
        min_text_chars: usize,
    },
    /// Extract and sync knowledge to vault via claude -p
    Sync {
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = "7")]
        days: u32,
        #[arg(long, default_value = "500")]
        min_text_chars: usize,
        #[arg(long, default_value = "sonnet")]
        model: String,
    },
    /// Output JSON status for the tray app
    Status,
    /// List unprocessed sessions
    List {
        #[arg(long, default_value = "7")]
        days: u32,
    },
    /// Show session interaction statistics
    Stats {
        /// Number of days to look back
        #[arg(long, default_value = "7")]
        days: u32,
        /// Show stats for a specific session
        #[arg(long)]
        session: Option<String>,
        /// Include code-to-commit ratio (requires git)
        #[arg(long)]
        commit_ratio: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List detected prompt files written by Claude
    Prompts {
        /// Number of days to look back
        #[arg(long, default_value = "30")]
        days: u32,
        /// Minimum confidence threshold (0.0 - 1.0)
        #[arg(long, default_value = "0.5")]
        threshold: f64,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show distributions and breakdowns over recent sessions
    Analytics {
        /// Number of days to look back
        #[arg(long, default_value = "30")]
        days: u32,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    // Configure logging: --verbose > --quiet > RUST_LOG env > default (warn)
    if cli.verbose {
        std::env::set_var("RUST_LOG", "debug");
    } else if cli.quiet {
        std::env::set_var("RUST_LOG", "off");
    } else if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "warn");
    }
    env_logger::init();

    let config = Config::load();

    let result: Result<()> = match cli.command {
        Commands::Extract {
            days,
            min_text_chars,
        } => commands::cmd_extract(&config, days, min_text_chars),
        Commands::Sync {
            dry_run,
            days,
            min_text_chars,
            model,
        } => commands::cmd_sync(&config, dry_run, days, min_text_chars, &model),
        Commands::Status => commands::cmd_status(&config),
        Commands::List { days } => commands::cmd_list(&config, days),
        Commands::Stats {
            days,
            session,
            commit_ratio,
            json,
        } => commands::cmd_stats(&config, days, session.as_deref(), commit_ratio, json),
        Commands::Prompts {
            days,
            threshold,
            json,
        } => commands::cmd_prompts(&config, days, threshold, json),
        Commands::Analytics { days, json } => commands::cmd_analytics(&config, days, json),
    };

    if let Err(e) = result {
        log::error!("{:#}", e);
        process::exit(1);
    }
}
