//! Summarize command - generate token-aware summary of recent logs
//!
//! Usage: logpilot summarize --last 10m

use chrono::{DateTime, Duration, Utc};
use clap::Args;
use std::collections::HashMap;

/// Summarize recent log activity
#[derive(Args, Clone)]
pub struct SummarizeArgs {
    /// Time window to summarize (e.g., 10m, 1h, 30s)
    #[arg(short, long, default_value = "10m")]
    pub last: String,

    /// Output format
    #[arg(short, long, default_value = "text")]
    pub format: String,

    /// Max tokens in output (approximate)
    #[arg(short, long, default_value = "4000")]
    pub tokens: usize,

    /// Show only errors and above
    #[arg(long)]
    pub errors_only: bool,
}

/// Handle the summarize command
pub async fn handle(args: SummarizeArgs) -> anyhow::Result<()> {
    // Parse duration
    let duration = parse_duration(&args.last)?;
    let window_start = Utc::now() - duration;

    println!("Generating summary for last {}...", args.last);
    println!(
        "Window: {} to {}",
        window_start.format("%Y-%m-%d %H:%M:%S UTC"),
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!();
    println!("Note: Full summarize integration requires active watch session.");
    println!("Run 'logpilot watch <session-name>' to start capturing logs.");

    // Placeholder summary
    let summary = generate_summary_placeholder(window_start, args.errors_only).await?;

    // Format output
    match args.format.as_str() {
        "json" => println!("{}", serde_json::to_string_pretty(&summary)?),
        _ => print_text_summary(&summary, args.tokens)?,
    }

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

/// Summary data structure
#[derive(Debug, Clone, serde::Serialize)]
pub struct Summary {
    pub session_name: String,
    pub generated_at: DateTime<Utc>,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub total_entries: usize,
    pub entries_by_severity: HashMap<String, usize>,
    pub active_incidents: Vec<IncidentSummary>,
    pub top_patterns: Vec<PatternSummary>,
    pub active_alerts: Vec<AlertSummary>,
    pub services_affected: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct IncidentSummary {
    pub id: String,
    pub title: String,
    pub severity: String,
    pub status: String,
    pub started_at: String,
    pub affected_services: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PatternSummary {
    pub id: String,
    pub signature: String,
    pub severity: String,
    pub occurrence_count: u64,
    pub window_count: u32,
    pub sample_message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AlertSummary {
    pub id: String,
    pub alert_type: String,
    pub message: String,
    pub status: String,
    pub triggered_at: String,
}

/// Generate placeholder summary
async fn generate_summary_placeholder(
    window_start: DateTime<Utc>,
    _errors_only: bool,
) -> anyhow::Result<Summary> {
    // Build summary with sample data
    let mut entries_by_severity = HashMap::new();
    entries_by_severity.insert("INFO".to_string(), 42);
    entries_by_severity.insert("WARN".to_string(), 5);
    entries_by_severity.insert("ERROR".to_string(), 2);

    let summary = Summary {
        session_name: "demo-session".to_string(),
        generated_at: Utc::now(),
        window_start,
        window_end: Utc::now(),
        total_entries: 49,
        entries_by_severity,
        active_incidents: Vec::new(),
        top_patterns: Vec::new(),
        active_alerts: Vec::new(),
        services_affected: vec!["api-service".to_string(), "db-service".to_string()],
    };

    Ok(summary)
}

/// Print summary in human-readable format
fn print_text_summary(summary: &Summary, max_tokens: usize) -> anyhow::Result<()> {
    let mut output = String::new();

    // Header
    output.push_str(&format!("Total Entries: {}\n", summary.total_entries));
    output.push_str(&format!(
        "Generated: {}\n",
        summary.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
    ));
    output.push('\n');

    // Severity breakdown
    if !summary.entries_by_severity.is_empty() {
        output.push_str("Severity Distribution:\n");
        let mut severities: Vec<_> = summary.entries_by_severity.iter().collect();
        severities.sort_by(|a, b| b.1.cmp(a.1)); // Sort by count descending

        for (sev, count) in severities {
            output.push_str(&format!("  {}: {}\n", sev, count));
        }
        output.push('\n');
    }

    // Active incidents
    if !summary.active_incidents.is_empty() {
        output.push_str("Active Incidents:\n");
        for incident in &summary.active_incidents {
            output.push_str(&format!(
                "  [{}] {} - {}\n",
                incident.severity, incident.title, incident.status
            ));
            if !incident.affected_services.is_empty() {
                output.push_str(&format!(
                    "    Services: {}\n",
                    incident.affected_services.join(", ")
                ));
            }
        }
        output.push('\n');
    }

    // Top patterns
    if !summary.top_patterns.is_empty() {
        output.push_str("Top Patterns:\n");
        for pattern in &summary.top_patterns {
            output.push_str(&format!(
                "  [{}] {} occurrences ({} in window)\n",
                pattern.severity, pattern.occurrence_count, pattern.window_count
            ));
            output.push_str(&format!(
                "    Sample: {}\n",
                pattern.sample_message.chars().take(80).collect::<String>()
            ));
        }
        output.push('\n');
    }

    // Active alerts
    if !summary.active_alerts.is_empty() {
        output.push_str("Active Alerts:\n");
        for alert in &summary.active_alerts {
            output.push_str(&format!(
                "  [{}] {} - {}\n",
                alert.alert_type, alert.message, alert.status
            ));
        }
        output.push('\n');
    }

    // Services affected
    if !summary.services_affected.is_empty() {
        output.push_str(&format!(
            "Services Affected: {}\n",
            summary.services_affected.join(", ")
        ));
    }

    // Token-aware truncation
    let estimated_tokens = output.len() / 4; // Rough approximation: ~4 chars per token
    if estimated_tokens > max_tokens {
        // Truncate output
        let truncate_at = max_tokens * 4;
        output.truncate(truncate_at);
        output.push_str("\n\n[Output truncated due to token limit]");
    }

    println!("{}", output);
    Ok(())
}
