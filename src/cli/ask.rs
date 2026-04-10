//! Ask command - build a ready-to-paste debugging prompt from real log data
//!
//! Usage:
//!   logpilot ask <session-name>
//!   logpilot ask <session-name> "Why are checkout requests failing?"
//!   logpilot ask <session-name> --last 30m

use chrono::{Duration, Utc};
use clap::Args;

use crate::capture::tmux::TmuxCommand;
use crate::models::{LogEntry, Severity};
use crate::pipeline::parser::LogParser;

/// Build a debugging prompt from log data for a tmux session
#[derive(Args, Clone)]
pub struct AskArgs {
    /// tmux session name to query (e.g. my-app, 2026-02-26-aircarbon)
    pub session: String,

    /// Optional question to include in the prompt
    pub question: Option<String>,

    /// Time window to include (e.g., 10m, 1h, 2h)
    #[arg(short, long, default_value = "30m")]
    pub last: String,

    /// Minimum severity level (error, fatal)
    #[arg(short = 'L', long, default_value = "error")]
    pub level: String,
}

pub async fn handle(args: AskArgs) -> anyhow::Result<()> {
    let duration = parse_duration(&args.last)?;
    let window_start = Utc::now() - duration;
    let window_end = Utc::now();

    // Parse severity level
    let min_severity = parse_level(&args.level);

    // Capture directly from tmux session (database doesn't track session name per entry)
    let entries = capture_from_tmux(&args.session, &window_start, min_severity).await?;

    // Build the prompt
    let mut prompt = String::new();

    prompt.push_str(&format!("# Debug session: `{}`\n\n", args.session));
    prompt.push_str(&format!(
        "**Time window:** {} → {} UTC  \n",
        window_start.format("%Y-%m-%d %H:%M:%S"),
        window_end.format("%Y-%m-%d %H:%M:%S"),
    ));
    prompt.push_str(&format!("**Session:** `{}`\n\n", args.session));

    if entries.is_empty() {
        prompt.push_str("## Logs\n\n");
        prompt.push_str(
            "> No ERROR or FATAL log entries found for this session in the given window.\n",
        );
        prompt.push_str(">\n");
        prompt.push_str(&format!(
            "> To capture logs, run: `logpilot watch {}`\n\n",
            args.session
        ));
    } else {
        // Severity breakdown
        let error_count = entries
            .iter()
            .filter(|e| e.severity == Severity::Error)
            .count();
        let fatal_count = entries
            .iter()
            .filter(|e| e.severity == Severity::Fatal)
            .count();

        prompt.push_str("## Summary\n\n");
        if fatal_count > 0 {
            prompt.push_str(&format!("- FATAL: {}\n", fatal_count));
        }
        prompt.push_str(&format!("- ERROR: {}\n\n", error_count));

        // Services mentioned
        let mut services: Vec<String> = entries
            .iter()
            .filter_map(|e| e.service.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        services.sort();
        if !services.is_empty() {
            prompt.push_str(&format!(
                "**Affected services:** {}\n\n",
                services.join(", ")
            ));
        }

        // Log entries — most recent first, cap at 50 to stay token-friendly
        prompt.push_str("## Error Logs\n\n");
        prompt.push_str("```\n");
        for entry in entries.iter().take(50) {
            let ts = entry.timestamp.format("%Y-%m-%d %H:%M:%S");
            let svc = entry
                .service
                .as_deref()
                .map(|s| format!("[{}] ", s))
                .unwrap_or_default();
            prompt.push_str(&format!(
                "[{}] {}{}: {}\n",
                ts,
                svc,
                entry.severity,
                entry.raw_content.trim()
            ));
        }
        if entries.len() > 50 {
            prompt.push_str(&format!(
                "... ({} more entries omitted)\n",
                entries.len() - 50
            ));
        }
        prompt.push_str("```\n\n");
    }

    // Question section
    if let Some(ref q) = args.question {
        prompt.push_str(&format!("## Question\n\n{}\n\n", q));
    } else {
        prompt.push_str("## Question\n\n");
        prompt.push_str("Please analyze the errors above and:\n");
        prompt.push_str("1. Identify the root cause\n");
        prompt.push_str("2. Suggest concrete fixes with code examples if relevant\n");
        prompt.push_str("3. Note any patterns (repeated failures, cascading errors, etc.)\n\n");
    }

    print!("{}", prompt);
    Ok(())
}

/// Capture logs directly from tmux session
async fn capture_from_tmux(
    session: &str,
    window_start: &chrono::DateTime<Utc>,
    min_severity: Severity,
) -> anyhow::Result<Vec<LogEntry>> {
    use tokio::process::Command;

    // Verify session exists
    if !TmuxCommand::session_exists(session).await? {
        return Err(anyhow::anyhow!("Session '{}' not found", session));
    }

    // Get all panes in the session
    let panes = TmuxCommand::list_panes(session).await?;
    if panes.is_empty() {
        return Err(anyhow::anyhow!("No panes found in session '{}'", session));
    }

    let mut entries = Vec::new();
    let mut sequence: u64 = 0;
    let parser = LogParser::new();

    // Helper to check if severity meets minimum
    fn severity_meets_min(sev: Severity, min: Severity) -> bool {
        // Only include specific severities that meet threshold
        // Unknown is excluded since it has highest discriminant
        matches!(
            (sev, min),
            (Severity::Fatal, Severity::Fatal)
                | (Severity::Fatal, Severity::Error)
                | (Severity::Error, Severity::Error)
        )
    }

    // Capture from each pane (last 1000 lines)
    for pane in panes {
        let output = Command::new("tmux")
            .args(["capture-pane", "-p", "-t", &pane, "-S", "-1000"])
            .output()
            .await?;

        if !output.status.success() {
            continue;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Create entry and parse it
            let mut entry =
                LogEntry::new(uuid::Uuid::nil(), sequence, Utc::now(), trimmed.to_string());
            parser.parse(&mut entry);
            sequence += 1;

            // Filter by minimum severity
            if severity_meets_min(entry.severity, min_severity) {
                entries.push(entry);
            }
        }
    }

    // Sort by timestamp (most recent first) and filter by time window
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    entries.retain(|e| e.timestamp >= *window_start);

    Ok(entries)
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
        .map_err(|_| anyhow::anyhow!("Invalid duration number in '{}'", s))?;
    let unit: String = chars.collect();

    match unit.as_str() {
        "s" => Ok(Duration::seconds(value)),
        "m" => Ok(Duration::minutes(value)),
        "h" => Ok(Duration::hours(value)),
        "d" => Ok(Duration::days(value)),
        _ => Err(anyhow::anyhow!(
            "Invalid duration unit '{}' in '{}' — use s, m, h, or d",
            unit,
            s
        )),
    }
}

/// Parse severity level from string (error, fatal)
fn parse_level(s: &str) -> Severity {
    match s.to_lowercase().as_str() {
        "fatal" => Severity::Fatal,
        "error" => Severity::Error,
        _ => Severity::Error, // default to error
    }
}
