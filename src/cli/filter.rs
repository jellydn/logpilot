//! Filter command - filter error lines from a tmux session
//!
//! Usage: logpilot filter <session> [--pane <pane>] [--level <level>] [--follow]

use crate::capture::tmux::TmuxCommand;
use crate::error::{LogPilotError, Result};
use crate::models::Severity;
use clap::Args;
use regex::Regex;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Filter error lines from a tmux session
#[derive(Args, Clone)]
pub struct FilterArgs {
    /// Name of the tmux session to filter
    pub session: String,
    /// Specific pane to filter (default: all panes)
    #[arg(short = 'P', long)]
    pub pane: Option<String>,
    /// Minimum severity level to filter (trace, debug, info, warn, error, fatal)
    #[arg(short = 'L', long, default_value = "error")]
    pub level: String,
    /// Follow output continuously (like tail -f)
    #[arg(short, long)]
    pub follow: bool,
    /// Additional pattern to filter (regex)
    #[arg(short = 'R', long)]
    pub pattern: Option<String>,
    /// Show N lines of context around matches
    #[arg(short = 'C', long, default_value = "0")]
    pub context: usize,
    /// Maximum number of lines to output
    #[arg(short = 'N', long)]
    pub limit: Option<usize>,
}

/// Handle the filter command
pub async fn handle(args: FilterArgs) -> Result<()> {
    // Validate session exists
    if !TmuxCommand::session_exists(&args.session).await? {
        return Err(LogPilotError::tmux(format!(
            "Session '{}' not found",
            args.session
        )));
    }

    // Parse severity level
    let min_severity = parse_severity(&args.level);

    // Compile pattern regex if provided
    let pattern_regex = if let Some(ref pattern) = args.pattern {
        Some(
            Regex::new(pattern)
                .map_err(|e| LogPilotError::config(format!("Invalid regex: {}", e)))?,
        )
    } else {
        None
    };

    // Get panes to filter
    let panes = if let Some(ref pane) = args.pane {
        vec![pane.clone()]
    } else {
        TmuxCommand::list_panes(&args.session).await?
    };

    if panes.is_empty() {
        return Err(LogPilotError::tmux(format!(
            "No panes found in session '{}'",
            args.session
        )));
    }

    println!(
        "🔍 Filtering {} level logs from '{}'",
        args.level.to_uppercase(),
        args.session
    );
    if let Some(ref pattern) = args.pattern {
        println!("   Pattern: {}", pattern);
    }
    println!("   Hint: Use -f to follow live, -R 'pattern' for regex filter, -N 10 to limit lines");
    println!();

    if args.follow {
        // Continuous mode - stream and filter
        run_filter_stream(&panes, min_severity, pattern_regex, args.context).await?;
    } else {
        // Snapshot mode - capture and filter current buffer
        run_filter_snapshot(
            &panes,
            min_severity,
            pattern_regex,
            args.context,
            args.limit,
        )
        .await?;
    }

    Ok(())
}

/// Parse severity level from string
pub fn parse_severity(level: &str) -> Severity {
    match level.to_lowercase().as_str() {
        "trace" => Severity::Trace,
        "debug" => Severity::Debug,
        "info" => Severity::Info,
        "warn" => Severity::Warn,
        "warning" => Severity::Warn,
        "error" => Severity::Error,
        "fatal" => Severity::Fatal,
        _ => Severity::Error,
    }
}

/// Check if a log line matches error criteria
pub fn line_matches(
    line: &str,
    min_severity: Severity,
    pattern_regex: &Option<Regex>,
) -> Option<Severity> {
    // Detect severity from line content
    let detected_severity = detect_severity(line);

    // Check if meets minimum severity
    if detected_severity < min_severity {
        // Also check if pattern matches anyway
        if let Some(ref regex) = pattern_regex {
            if regex.is_match(line) {
                return Some(detected_severity);
            }
        }
        return None;
    }

    // Check pattern if specified
    if let Some(ref regex) = pattern_regex {
        if !regex.is_match(line) {
            return None;
        }
    }

    Some(detected_severity)
}

/// Detect severity from log line content
pub fn detect_severity(line: &str) -> Severity {
    let line_lower = line.to_lowercase();

    // Check for fatal/crash indicators
    if line_lower.contains("fatal")
        || line_lower.contains("panic")
        || line_lower.contains("crash")
        || line_lower.contains("sigsegv")
        || line_lower.contains("sigkill")
    {
        return Severity::Fatal;
    }

    // Check for error indicators
    if line_lower.contains("error")
        || line_lower.contains("exception")
        || line_lower.contains("fail")
        || line_lower.contains("failed")
        || line_lower.contains("errno")
        || line_lower.contains("econnrefused")
        || line_lower.contains("econnreset")
        || line_lower.contains("eaddrnotavail")
    {
        return Severity::Error;
    }

    // Check for warning indicators
    if line_lower.contains("warn")
        || line_lower.contains("warning")
        || line_lower.contains("deprecated")
        || line_lower.contains("caution")
    {
        return Severity::Warn;
    }

    // Check for info/debug/trace patterns
    if line_lower.contains("info") || line_lower.contains("[inf]") {
        return Severity::Info;
    }
    if line_lower.contains("debug")
        || line_lower.contains("[dbg]")
        || line_lower.contains("[debug]")
    {
        return Severity::Debug;
    }
    if line_lower.contains("trace") || line_lower.contains("[trc]") {
        return Severity::Trace;
    }

    Severity::Unknown
}

/// Get icon for severity
fn get_severity_icon(severity: Severity) -> &'static str {
    match severity {
        Severity::Trace => "⚪",
        Severity::Debug => "🔵",
        Severity::Info => "💙",
        Severity::Warn => "🟡",
        Severity::Error => "🔴",
        Severity::Fatal => "💥",
        Severity::Unknown => "⚫",
    }
}

/// Run filter in snapshot mode (capture current buffer)
async fn run_filter_snapshot(
    panes: &[String],
    min_severity: Severity,
    pattern_regex: Option<Regex>,
    _context: usize,
    limit: Option<usize>,
) -> Result<()> {
    let mut total_matches = 0;
    let max_lines = limit.unwrap_or(1000);

    for pane in panes {
        // Capture pane history
        let output = Command::new("tmux")
            .args(["capture-pane", "-p", "-t", pane, "-S", "-1000"])
            .output()
            .await
            .map_err(LogPilotError::Io)?;

        if !output.status.success() {
            continue;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        for line in lines {
            if let Some(severity) = line_matches(line, min_severity, &pattern_regex) {
                let icon = get_severity_icon(severity);
                println!(
                    "{} [{}] {}",
                    icon,
                    &pane[..pane.len().min(8)],
                    line.chars().take(120).collect::<String>()
                );
                total_matches += 1;

                if total_matches >= max_lines {
                    println!("\n... (limit reached: {} lines)", max_lines);
                    return Ok(());
                }
            }
        }
    }

    println!("\n📊 Found {} matching lines", total_matches);
    Ok(())
}

/// Run filter in streaming mode (follow output)
async fn run_filter_stream(
    panes: &[String],
    min_severity: Severity,
    pattern_regex: Option<Regex>,
    _context: usize,
) -> Result<()> {
    use std::path::PathBuf;
    use tokio::sync::mpsc;

    let (tx, mut rx) = mpsc::unbounded_channel::<(String, String)>();
    let pane_count = panes.len();

    // Track FIFO paths for cleanup
    let mut fifo_paths: Vec<PathBuf> = Vec::new();

    // Spawn capture tasks for each pane using FIFO approach
    for pane in panes {
        let tx = tx.clone();
        let pane_clone = pane.clone();

        // Create FIFO for this pane
        let fifo_path = std::env::temp_dir().join(format!(
            "logpilot-filter-{}-{}-{}.fifo",
            pane.replace('%', "p"),
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        fifo_paths.push(fifo_path.clone());

        // Create FIFO
        tokio::process::Command::new("mkfifo")
            .arg(&fifo_path)
            .output()
            .await
            .map_err(LogPilotError::Io)?;

        // Start pipe-pane to redirect output to FIFO
        let fifo_str = fifo_path.to_string_lossy().to_string();
        let cmd = format!("exec cat >> '{}'", fifo_str.replace('\'', "'\"'\"'"));
        let pipe_result = Command::new("tmux")
            .args(["pipe-pane", "-t", &pane_clone, &cmd])
            .output()
            .await;

        if let Err(e) = pipe_result {
            eprintln!("⚠️  Failed to start pipe-pane for {}: {}", pane_clone, e);
            let _ = tokio::fs::remove_file(&fifo_path).await;
            continue; // Skip this pane and continue with others
        }

        tokio::spawn(async move {
            // Open FIFO and read lines
            loop {
                let file = match File::open(&fifo_path).await {
                    Ok(f) => f,
                    Err(_) => {
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        continue;
                    }
                };

                let reader = BufReader::new(file);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = tx.send((pane_clone.clone(), line));
                }

                // FIFO closed, retry after brief delay
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
    }

    println!(
        "📡 Streaming from {} pane(s)... Press Ctrl+C to stop\n",
        pane_count
    );

    // Process incoming lines
    let result = loop {
        tokio::select! {
            Some((pane, line)) = rx.recv() => {
                if let Some(severity) = line_matches(&line, min_severity, &pattern_regex) {
                    let icon = get_severity_icon(severity);
                    let pane_short = pane.chars().take(8).collect::<String>();
                    println!("{} [{}] {}", icon, pane_short, line.chars().take(120).collect::<String>());
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\n\nStopping...");
                break Ok(());
            }
        }
    };

    // Cleanup: stop pipe-pane and remove FIFOs
    for (pane, fifo_path) in panes.iter().zip(fifo_paths.iter()) {
        let _ = Command::new("tmux")
            .args(["pipe-pane", "-t", pane])
            .output()
            .await;
        let _ = tokio::fs::remove_file(fifo_path).await;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_severity() {
        assert_eq!(parse_severity("error"), Severity::Error);
        assert_eq!(parse_severity("ERROR"), Severity::Error);
        assert_eq!(parse_severity("warn"), Severity::Warn);
        assert_eq!(parse_severity("warning"), Severity::Warn);
        assert_eq!(parse_severity("fatal"), Severity::Fatal);
        assert_eq!(parse_severity("debug"), Severity::Debug);
        assert_eq!(parse_severity("unknown"), Severity::Error); // Default
    }

    #[test]
    fn test_detect_severity() {
        assert_eq!(detect_severity("ERROR: something failed"), Severity::Error);
        assert_eq!(detect_severity("FATAL: process crashed"), Severity::Fatal);
        assert_eq!(detect_severity("WARN: deprecated usage"), Severity::Warn);
        assert_eq!(detect_severity("INFO: started service"), Severity::Info);
        assert_eq!(detect_severity("DEBUG: variable x=42"), Severity::Debug);
        assert_eq!(detect_severity("random log line"), Severity::Unknown);
    }

    #[test]
    fn test_line_matches() {
        let regex: Option<Regex> = None;
        assert!(line_matches("ERROR: test", Severity::Error, &regex).is_some());
        assert!(line_matches("DEBUG: test", Severity::Error, &regex).is_none());
        assert!(line_matches("INFO: test", Severity::Info, &regex).is_some());
    }

    #[test]
    fn test_line_matches_with_pattern() {
        let regex = Some(Regex::new("database").unwrap());
        assert!(
            line_matches("ERROR: database connection failed", Severity::Error, &regex).is_some()
        );
        assert!(line_matches("DEBUG: cache hit", Severity::Debug, &regex).is_none());
        // Pattern matches even if severity is below threshold
        assert!(line_matches("INFO: database query", Severity::Error, &regex).is_some());
    }
}
