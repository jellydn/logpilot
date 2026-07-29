//! Summarize command - generate token-aware summary of recent logs
//!
//! Usage: logpilot summarize --last 10m

use crate::mcp::data_store::{get_or_init_global_store, SessionDataStore};
use crate::models::{AlertStatus, IncidentStatus};
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

    // Try to read live data from the global data store (populated by watch command)
    let store = get_or_init_global_store();
    let sessions = store.list_sessions();

    if sessions.is_empty() {
        println!("No active watch sessions found.");
        println!("Run 'logpilot watch <session-name>' to start capturing logs.");
        println!();
        println!("Generating placeholder summary for last {}...", args.last);
    }

    // Generate summary from the first available session, or use placeholder
    let summary = if let Some(session_name) = sessions.first() {
        generate_summary_from_store(&store, session_name, window_start).await?
    } else {
        generate_summary_placeholder(window_start, args.errors_only).await?
    };

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

/// Generate summary from the live data store
async fn generate_summary_from_store(
    store: &SessionDataStore,
    session_name: &str,
    window_start: DateTime<Utc>,
) -> anyhow::Result<Summary> {
    let session = store
        .get_session(session_name)
        .await
        .ok_or_else(|| anyhow::anyhow!("Session '{}' not found in data store", session_name))?;

    // Filter entries within time window
    let window_entries: Vec<_> = session
        .entries
        .iter()
        .filter(|e| e.timestamp >= window_start)
        .collect();

    // Severity distribution
    let mut entries_by_severity = HashMap::new();
    let mut services = HashMap::new();
    for entry in &window_entries {
        *entries_by_severity
            .entry(format!("{:?}", entry.severity))
            .or_insert(0) += 1;
        if let Some(ref svc) = entry.service {
            *services.entry(svc.clone()).or_insert(0) += 1;
        }
    }

    let services_affected: Vec<String> = {
        let mut svcs: Vec<_> = services.into_iter().collect();
        svcs.sort_by_key(|b| std::cmp::Reverse(b.1));
        svcs.into_iter().map(|(s, _)| s).collect()
    };

    // Patterns from data store
    let top_patterns: Vec<PatternSummary> = session
        .patterns
        .iter()
        .take(10)
        .map(|p| PatternSummary {
            id: p.id.to_string(),
            signature: p.signature.clone(),
            severity: format!("{:?}", p.severity),
            occurrence_count: p.occurrence_count,
            window_count: p.window_count,
            sample_message: p
                .sample_entry
                .map(|id| id.to_string())
                .unwrap_or_else(|| "(no sample)".to_string()),
        })
        .collect();

    // Incidents from data store
    let active_incidents: Vec<IncidentSummary> = session
        .incidents
        .iter()
        .filter(|i| i.status == IncidentStatus::Active)
        .map(|i| IncidentSummary {
            id: i.id.to_string(),
            title: i.title.clone(),
            severity: format!("{:?}", i.severity),
            status: format!("{:?}", i.status),
            started_at: i.started_at.to_rfc3339(),
            affected_services: i.affected_services.clone(),
        })
        .collect();

    // Alerts from data store
    let active_alerts: Vec<AlertSummary> = session
        .alerts
        .iter()
        .filter(|a| a.status == AlertStatus::Active)
        .map(|a| AlertSummary {
            id: a.id.to_string(),
            alert_type: format!("{:?}", a.alert_type),
            message: a.message.clone(),
            status: format!("{:?}", a.status),
            triggered_at: a.triggered_at.to_rfc3339(),
        })
        .collect();

    Ok(Summary {
        session_name: session_name.to_string(),
        generated_at: Utc::now(),
        window_start,
        window_end: Utc::now(),
        total_entries: window_entries.len(),
        entries_by_severity,
        active_incidents,
        top_patterns,
        active_alerts,
        services_affected,
    })
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
        severities.sort_by_key(|b| std::cmp::Reverse(*b.1)); // Sort by count descending

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
