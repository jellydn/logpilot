use clap::{Parser, Subcommand};
use tracing::info;

mod analyzer;
mod buffer;
mod capture;
mod cli;
mod error;
mod mcp;
mod models;
mod pipeline;

use error::Result;

#[derive(Parser)]
#[command(name = "logpilot")]
#[command(about = "AI-Native tmux Log Copilot for Support Incident Tracking")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Watch a tmux session and capture logs
    Watch {
        /// Name of the tmux session to watch
        session: String,
        /// Specific pane to watch (default: active pane)
        #[arg(short, long)]
        pane: Option<String>,
        /// Rolling buffer duration in minutes
        #[arg(short, long, default_value = "30")]
        buffer: u32,
        /// Minimum severity level to display (trace, debug, info, warn, error, fatal)
        #[arg(short = 'L', long, default_value = "warn")]
        level: String,
    },
    /// Filter error lines from a tmux session
    Filter {
        /// Name of the tmux session to filter
        session: String,
        /// Specific pane to filter (default: all panes)
        #[arg(short = 'P', long)]
        pane: Option<String>,
        /// Minimum severity level (trace, debug, info, warn, error, fatal)
        #[arg(short = 'L', long, default_value = "error")]
        level: String,
        /// Follow output continuously
        #[arg(short, long)]
        follow: bool,
        /// Additional pattern to filter (regex)
        #[arg(short = 'R', long)]
        pattern: Option<String>,
        /// Lines of context around matches
        #[arg(short = 'C', long, default_value = "0")]
        context: usize,
        /// Maximum number of lines to output
        #[arg(short = 'N', long)]
        limit: Option<usize>,
    },
    /// Summarize recent log activity
    Summarize {
        /// Time window to summarize (e.g., "10m", "1h")
        #[arg(short, long)]
        last: String,
    },
    /// Build a debugging prompt from log data for a session
    Ask {
        /// tmux session name to query
        session: String,
        /// Optional question to include in the prompt
        question: Option<String>,
        /// Time window (e.g., 10m, 1h)
        #[arg(short, long, default_value = "30m")]
        last: String,
        /// Minimum severity level (error, fatal)
        #[arg(short = 'L', long, default_value = "error")]
        level: String,
    },
    /// Start MCP server mode
    McpServer {
        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,
    },
    /// Show status of monitored sessions
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Watch {
            session,
            pane,
            buffer,
            level,
        } => {
            let options = cli::watch::WatchOptions {
                session,
                pane,
                buffer_minutes: buffer,
                level,
            };
            cli::watch::run(options).await?;
        }
        Commands::Filter {
            session,
            pane,
            level,
            follow,
            pattern,
            context,
            limit,
        } => {
            let args = cli::filter::FilterArgs {
                session,
                pane,
                level,
                follow,
                pattern,
                context,
                limit,
            };
            if let Err(e) = cli::filter::handle(args).await {
                eprintln!("Error: {}", e);
            }
        }
        Commands::Summarize { last } => {
            info!("Generating summary for last {}", last);
            let args = cli::summarize::SummarizeArgs {
                last,
                format: "text".to_string(),
                tokens: 4000,
                errors_only: false,
            };
            if let Err(e) = cli::summarize::handle(args).await {
                eprintln!("Error: {}", e);
            }
        }
        Commands::Ask {
            session,
            question,
            last,
            level,
        } => {
            let args = cli::ask::AskArgs {
                session,
                question,
                last,
                level,
            };
            if let Err(e) = cli::ask::handle(args).await {
                eprintln!("Error: {}", e);
            }
        }
        Commands::McpServer { verbose } => {
            if verbose {
                info!("Starting MCP server in verbose mode");
            } else {
                info!("Starting MCP server");
            }
            let args = cli::mcp::McpArgs {
                verbose,
                legacy: false,
            };
            if let Err(e) = cli::mcp::handle(args).await {
                eprintln!("Error: {}", e);
            }
        }
        Commands::Status => {
            info!("Fetching status");
            let args = cli::status::StatusArgs {
                detailed: false,
                session: None,
            };
            if let Err(e) = cli::status::handle(args).await {
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}
