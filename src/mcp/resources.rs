//! MCP Resource handlers for LogPilot
//!
//! Exposes session data via MCP resource URIs:
//! - logpilot://session/{name}/summary
//! - logpilot://session/{name}/entries
//! - logpilot://session/{name}/patterns
//! - logpilot://session/{name}/incidents
//! - logpilot://session/{name}/alerts

use crate::mcp::protocol::{Resource, ResourceContent, ResourcesListResult};
use crate::models::{Incident, LogEntry, Pattern};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::collections::HashMap;

/// Resource URI parser and handler
pub struct ResourceHandler;

impl ResourceHandler {
    /// List all available MCP resources
    pub fn list_resources() -> ResourcesListResult {
        ResourcesListResult {
            resources: vec![
                Resource {
                    uri: "logpilot://session/{name}/summary".to_string(),
                    name: "Session Summary".to_string(),
                    description: Some("Current incident summary for the session".to_string()),
                    mime_type: Some("application/json".to_string()),
                },
                Resource {
                    uri: "logpilot://session/{name}/entries".to_string(),
                    name: "Log Entries".to_string(),
                    description: Some("Log entries within a time range".to_string()),
                    mime_type: Some("application/json".to_string()),
                },
                Resource {
                    uri: "logpilot://session/{name}/patterns".to_string(),
                    name: "Detected Patterns".to_string(),
                    description: Some("Detected error patterns for the session".to_string()),
                    mime_type: Some("application/json".to_string()),
                },
                Resource {
                    uri: "logpilot://session/{name}/incidents".to_string(),
                    name: "Active Incidents".to_string(),
                    description: Some("Currently active incidents".to_string()),
                    mime_type: Some("application/json".to_string()),
                },
                Resource {
                    uri: "logpilot://session/{name}/alerts".to_string(),
                    name: "Active Alerts".to_string(),
                    description: Some("Currently firing alerts".to_string()),
                    mime_type: Some("application/json".to_string()),
                },
            ],
        }
    }

    /// Parse a resource URI and extract components
    pub fn parse_uri(uri: &str) -> Option<ParsedUri> {
        let prefix = "logpilot://session/";
        if !uri.starts_with(prefix) {
            return None;
        }

        let rest = &uri[prefix.len()..];
        let parts: Vec<&str> = rest.split('/').collect();

        if parts.len() < 2 {
            return None;
        }

        let session_name = parts[0].to_string();

        // Resource type may include query params, e.g., "entries?since=..."
        let resource_part = parts[1];
        let (resource_type, query_params) = if let Some(q_pos) = resource_part.find('?') {
            let rt = resource_part[..q_pos].to_string();
            let qp = Self::parse_query(&resource_part[q_pos + 1..]);
            (rt, qp)
        } else {
            (resource_part.to_string(), HashMap::new())
        };

        Some(ParsedUri {
            session_name,
            resource_type,
            query_params,
        })
    }

    fn parse_query(query: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        for part in query.split('&') {
            if let Some(eq) = part.find('=') {
                let key = part[..eq].to_string();
                let value = part[eq + 1..].to_string();
                params.insert(key, value);
            }
        }
        params
    }

    /// Build summary resource content
    pub fn build_summary(
        session_name: &str,
        entries: &[LogEntry],
        patterns: &[Pattern],
        incidents: &[Incident],
        alerts: &[crate::models::Alert],
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
    ) -> ResourceContent {
        // Count entries by severity
        let mut entries_by_severity: HashMap<String, u64> = HashMap::new();
        for entry in entries {
            let key = format!("{:?}", entry.severity).to_uppercase();
            *entries_by_severity.entry(key).or_insert(0) += 1;
        }

        // Build pattern summaries
        let pattern_summaries: Vec<serde_json::Value> = patterns
            .iter()
            .map(|p| {
                json!({
                    "id": p.id.to_string(),
                    "signature": p.signature.clone(),
                    "severity": format!("{:?}", p.severity).to_uppercase(),
                    "occurrence_count": p.occurrence_count,
                    "window_count": p.window_count,
                    "first_seen": p.first_seen.to_rfc3339(),
                    "last_seen": p.last_seen.to_rfc3339(),
                })
            })
            .collect();

        // Build alert summaries
        let alert_summaries: Vec<serde_json::Value> = alerts
            .iter()
            .map(|a| {
                json!({
                    "id": a.id.to_string(),
                    "type": format!("{:?}", a.alert_type),
                    "triggered_at": a.triggered_at.to_rfc3339(),
                    "status": format!("{:?}", a.status),
                    "message": a.message.clone(),
                    "threshold": a.threshold,
                    "current_value": a.current_value,
                })
            })
            .collect();

        // Collect affected services
        let services: Vec<String> = entries
            .iter()
            .filter_map(|e| e.service.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let summary = json!({
            "session_name": session_name,
            "generated_at": Utc::now().to_rfc3339(),
            "window_start": window_start.to_rfc3339(),
            "window_end": window_end.to_rfc3339(),
            "total_entries": entries.len(),
            "entries_by_severity": entries_by_severity,
            "active_incidents": incidents.len(),
            "top_patterns": pattern_summaries,
            "active_alerts": alert_summaries,
            "services_affected": services,
        });

        ResourceContent {
            uri: format!("logpilot://session/{}/summary", session_name),
            mime_type: Some("application/json".to_string()),
            text: summary.to_string(),
        }
    }

    /// Build entries resource content with optional filtering
    pub fn build_entries(
        session_name: &str,
        entries: &[LogEntry],
        query_params: &HashMap<String, String>,
    ) -> ResourceContent {
        use chrono::DateTime;

        // Parse filter parameters
        let severity_filter = query_params.get("severity").map(|s| s.to_uppercase());
        let service_filter = query_params.get("service").cloned();
        let limit = query_params
            .get("limit")
            .and_then(|l| l.parse::<usize>().ok())
            .map(|l| l.min(1000)) // Clamp to max 1000
            .unwrap_or(100);
        let offset = query_params
            .get("offset")
            .and_then(|o| o.parse::<usize>().ok())
            .unwrap_or(0);

        // Parse time range filters
        let since = query_params
            .get("since")
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let until = query_params
            .get("until")
            .and_then(|u| DateTime::parse_from_rfc3339(u).ok())
            .map(|dt| dt.with_timezone(&Utc));

        // Filter entries
        let filtered: Vec<&LogEntry> = entries
            .iter()
            .filter(|e| {
                // Time range filter (since)
                if let Some(since_dt) = since {
                    if e.timestamp < since_dt {
                        return false;
                    }
                }
                // Time range filter (until)
                if let Some(until_dt) = until {
                    if e.timestamp > until_dt {
                        return false;
                    }
                }
                // Severity filter
                if let Some(ref sev) = severity_filter {
                    let entry_sev = format!("{:?}", e.severity).to_uppercase();
                    if !entry_sev.contains(sev) {
                        return false;
                    }
                }
                // Service filter
                if let Some(ref svc) = service_filter {
                    if e.service.as_ref() != Some(svc) {
                        return false;
                    }
                }
                true
            })
            .skip(offset)
            .take(limit)
            .collect();

        let entries_json: Vec<serde_json::Value> = filtered
            .iter()
            .map(|e| {
                json!({
                    "id": e.id.to_string(),
                    "sequence": e.sequence,
                    "timestamp": e.timestamp.to_rfc3339(),
                    "severity": format!("{:?}", e.severity).to_uppercase(),
                    "service": e.service,
                    "raw_content": e.raw_content,
                    "parsed_fields": e.parsed_fields,
                })
            })
            .collect();

        // Calculate total after filtering (before pagination)
        let filtered_total = entries
            .iter()
            .filter(|e| {
                // Time range filter (since)
                if let Some(since_dt) = since {
                    if e.timestamp < since_dt {
                        return false;
                    }
                }
                // Time range filter (until)
                if let Some(until_dt) = until {
                    if e.timestamp > until_dt {
                        return false;
                    }
                }
                if let Some(ref sev) = severity_filter {
                    let entry_sev = format!("{:?}", e.severity).to_uppercase();
                    if !entry_sev.contains(sev) {
                        return false;
                    }
                }
                if let Some(ref svc) = service_filter {
                    if e.service.as_ref() != Some(svc) {
                        return false;
                    }
                }
                true
            })
            .count();

        let result = json!({
            "entries": entries_json,
            "pagination": {
                "total": filtered_total,
                "returned": entries_json.len(),
                "limit": limit,
                "offset": offset,
            },
            "filters": {
                "severity": severity_filter,
                "service": service_filter,
                "since": query_params.get("since").cloned(),
                "until": query_params.get("until").cloned(),
            }
        });

        // Build original URI with query params for accurate response
        let mut uri = format!("logpilot://session/{}/entries", session_name);
        if !query_params.is_empty() {
            let query = query_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            uri.push('?');
            uri.push_str(&query);
        }

        ResourceContent {
            uri,
            mime_type: Some("application/json".to_string()),
            text: result.to_string(),
        }
    }

    /// Build patterns resource content
    pub fn build_patterns(session_name: &str, patterns: &[Pattern]) -> ResourceContent {
        let patterns_json: Vec<serde_json::Value> = patterns
            .iter()
            .map(|p| {
                json!({
                    "id": p.id.to_string(),
                    "signature": p.signature.clone(),
                    "severity": format!("{:?}", p.severity).to_uppercase(),
                    "occurrence_count": p.occurrence_count,
                    "window_count": p.window_count,
                    "first_seen": p.first_seen.to_rfc3339(),
                    "last_seen": p.last_seen.to_rfc3339(),
                    "sample_entry": p.sample_entry.map(|id| id.to_string()),
                })
            })
            .collect();

        ResourceContent {
            uri: format!("logpilot://session/{}/patterns", session_name),
            mime_type: Some("application/json".to_string()),
            text: serde_json::to_string(&patterns_json).unwrap_or_default(),
        }
    }

    /// Build incidents resource content
    pub fn build_incidents(session_name: &str, incidents: &[Incident]) -> ResourceContent {
        let incidents_json: Vec<serde_json::Value> = incidents
            .iter()
            .map(|i| {
                json!({
                    "id": i.id.to_string(),
                    "title": i.title.clone(),
                    "severity": format!("{:?}", i.severity).to_uppercase(),
                    "status": format!("{:?}", i.status),
                    "started_at": i.started_at.to_rfc3339(),
                    "resolved_at": i.resolved_at.map(|t| t.to_rfc3339()),
                    "affected_services": i.affected_services,
                    "pattern_count": i.pattern_ids.len(),
                })
            })
            .collect();

        ResourceContent {
            uri: format!("logpilot://session/{}/incidents", session_name),
            mime_type: Some("application/json".to_string()),
            text: serde_json::to_string(&incidents_json).unwrap_or_default(),
        }
    }

    /// Build alerts resource content
    pub fn build_alerts(session_name: &str, alerts: &[crate::models::Alert]) -> ResourceContent {
        let alerts_json: Vec<serde_json::Value> = alerts
            .iter()
            .map(|a| {
                json!({
                    "id": a.id.to_string(),
                    "type": format!("{:?}", a.alert_type),
                    "triggered_at": a.triggered_at.to_rfc3339(),
                    "status": format!("{:?}", a.status),
                    "message": a.message.clone(),
                    "threshold": a.threshold,
                    "current_value": a.current_value,
                })
            })
            .collect();

        ResourceContent {
            uri: format!("logpilot://session/{}/alerts", session_name),
            mime_type: Some("application/json".to_string()),
            text: serde_json::to_string(&alerts_json).unwrap_or_default(),
        }
    }
}

/// Parsed URI components
#[derive(Debug, Clone)]
pub struct ParsedUri {
    pub session_name: String,
    pub resource_type: String,
    pub query_params: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Alert, AlertStatus, AlertType, Severity};
    use uuid::Uuid;

    #[test]
    fn test_parse_uri_summary() {
        let uri = "logpilot://session/test-session/summary";
        let parsed = ResourceHandler::parse_uri(uri).unwrap();
        assert_eq!(parsed.session_name, "test-session");
        assert_eq!(parsed.resource_type, "summary");
    }

    #[test]
    fn test_parse_uri_entries_with_query() {
        let uri = "logpilot://session/test-session/entries?since=2024-01-01T00:00:00Z";
        let parsed = ResourceHandler::parse_uri(uri).unwrap();
        assert_eq!(parsed.session_name, "test-session");
        assert_eq!(parsed.resource_type, "entries");
        assert_eq!(
            parsed.query_params.get("since"),
            Some(&"2024-01-01T00:00:00Z".to_string())
        );
    }

    #[test]
    fn test_list_resources() {
        let list = ResourceHandler::list_resources();
        assert_eq!(list.resources.len(), 5);
        assert!(list.resources.iter().any(|r| r.uri.contains("summary")));
    }

    #[test]
    fn test_build_summary() {
        let entry = LogEntry {
            id: Uuid::new_v4(),
            pane_id: Uuid::new_v4(),
            sequence: 1,
            timestamp: Utc::now(),
            severity: Severity::Error,
            service: Some("test-service".to_string()),
            raw_content: "test error".to_string(),
            parsed_fields: HashMap::new(),
            received_at: Utc::now(),
        };

        let alert = Alert {
            id: Uuid::new_v4(),
            alert_type: AlertType::RecurringError,
            incident_id: None,
            pattern_id: None,
            threshold: Some(5.0),
            current_value: 6.0,
            triggered_at: Utc::now(),
            acknowledged_at: None,
            status: AlertStatus::Active,
            message: "Test alert".to_string(),
            severity: Severity::Error,
        };

        let content = ResourceHandler::build_summary(
            "test-session",
            &[entry],
            &[],
            &[],
            &[alert],
            Utc::now() - chrono::Duration::minutes(10),
            Utc::now(),
        );

        assert!(content.uri.contains("summary"));
        assert!(content.text.contains("test-session"));
        assert!(content.text.contains("ERROR"));
    }
}
