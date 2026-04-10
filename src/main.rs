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
    },
    /// Summarize recent log activity
    Summarize {
        /// Time window to summarize (e.g., "10m", "1h")
        #[arg(short, long)]
        last: String,
    },
    /// Ask a question about the logs (AI-assisted)
    Ask {
        /// Question to ask about the logs
        question: String,
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
        } => {
            let options = cli::watch::WatchOptions {
                session,
                pane,
                buffer_minutes: buffer,
            };
            cli::watch::run(options).await?;
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
        Commands::Ask { question } => {
            info!("Processing question: {}", question);
            let args = cli::ask::AskArgs {
                question: vec![question],
                context: "10m".to_string(),
                include_logs: false,
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
            let args = cli::mcp::McpArgs { verbose };
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
