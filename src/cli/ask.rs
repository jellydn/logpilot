//! Ask command - format query with context for LLM
//!
//! Usage: logpilot ask "Why are checkout requests failing?"

use chrono::{Duration, Utc};
use clap::Args;

/// Ask AI-assisted questions about logs
#[derive(Args, Clone)]
pub struct AskArgs {
    /// The question to ask
    pub question: Vec<String>,

    /// Time window for context (e.g., 10m, 1h)
    #[arg(short, long, default_value = "10m")]
    pub context: String,

    /// Include raw log entries in context
    #[arg(short, long)]
    pub include_logs: bool,
}

/// Handle the ask command
pub async fn handle(args: AskArgs) -> anyhow::Result<()> {
    // Combine question parts
    let question = args.question.join(" ");

    if question.is_empty() {
        anyhow::bail!(
            "Please provide a question. Example: logpilot ask 'Why are errors increasing?'"
        );
    }

    // Parse context duration
    let duration = parse_duration(&args.context)?;
    let window_start = Utc::now() - duration;

    // Build context for LLM
    let mut context = String::new();
    context.push_str("# LogPilot Context\n\n");
    context.push_str(&format!(
        "Time Window: {} to {}\n",
        window_start.format("%Y-%m-%d %H:%M:%S UTC"),
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    // Add session placeholder
    context.push_str("\n## Active Sessions\n");
    context.push_str("(Run 'logpilot watch <session>' to capture logs)\n");

    // This would integrate with actual buffer data
    context.push_str("\n## Log Summary\n");
    context.push_str("(Integrate with actual buffer data in production)\n");

    if args.include_logs {
        context.push_str("\n## Recent Log Entries\n");
        context.push_str("(Would include relevant log entries here)\n");
    }

    // Format the final prompt
    let prompt = format!(
        "{context}\n\n## Question\n{question}\n\nPlease analyze the logs and provide insights."
    );

    // Output the formatted prompt
    println!("{}", "=".repeat(60));
    println!("Formatted query for Claude/Codex:");
    println!("{}", "=".repeat(60));
    println!();
    println!("{}", prompt);
    println!();
    println!("{}", "=".repeat(60));
    println!("Copy the above and paste into Claude Code or your preferred LLM.");
    println!("Future versions will support direct MCP integration.");

    Ok(())
}

/// Parse duration string (e.g., "10m", "1h", "30s")
fn parse_duration(s: &str) -> anyhow::Result<Duration> {
    let mut chars = s.chars().peekable();
    let mut num = String::new();

    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            num.push(c);
            chars.next();
        } else {
            break;
        }
    }

    let value: i64 = num
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid duration number"))?;
    let unit: String = chars.collect();

    match unit.as_str() {
        "s" => Ok(Duration::seconds(value)),
        "m" => Ok(Duration::minutes(value)),
        "h" => Ok(Duration::hours(value)),
        "d" => Ok(Duration::days(value)),
        _ => Err(anyhow::anyhow!("Invalid duration unit: {}", unit)),
    }
}
