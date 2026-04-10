use super::severity::Severity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: Uuid,
    pub pane_id: Uuid,
    pub sequence: u64,
    pub timestamp: DateTime<Utc>,
    pub severity: Severity,
    pub service: Option<String>,
    pub raw_content: String,
    pub parsed_fields: HashMap<String, String>,
    pub received_at: DateTime<Utc>,
}

#[allow(dead_code)]
impl LogEntry {
    pub fn new(
        pane_id: Uuid,
        sequence: u64,
        timestamp: DateTime<Utc>,
        raw_content: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            pane_id,
            sequence,
            timestamp,
            severity: Severity::Unknown,
            service: None,
            raw_content: raw_content.into(),
            parsed_fields: HashMap::new(),
            received_at: Utc::now(),
        }
    }

    pub fn new_with_severity(
        pane_id: Uuid,
        sequence: u64,
        timestamp: DateTime<Utc>,
        raw_content: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            pane_id,
            sequence,
            timestamp,
            severity,
            service: None,
            raw_content: raw_content.into(),
            parsed_fields: HashMap::new(),
            received_at: Utc::now(),
        }
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_service(mut self, service: impl Into<String>) -> Self {
        self.service = Some(service.into());
        self
    }

    pub fn with_parsed_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.parsed_fields.insert(key.into(), value.into());
        self
    }

    pub fn is_severe(&self) -> bool {
        matches!(self.severity, Severity::Error | Severity::Fatal)
    }

    /// Extract a signature for deduplication
    pub fn signature(&self) -> String {
        // Simple signature: severity + first 100 chars of normalized content
        let normalized = self
            .raw_content
            .replace(|c: char| c.is_whitespace(), " ")
            .trim()
            .to_lowercase();

        let content_hash = if normalized.len() > 100 {
            &normalized[..100]
        } else {
            &normalized
        };

        format!("{}:{}", self.severity, content_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry_creation() {
        let pane_id = Uuid::new_v4();
        let timestamp = Utc::now();
        let entry = LogEntry::new(pane_id, 1, timestamp, "Test log message");

        assert_eq!(entry.sequence, 1);
        assert_eq!(entry.severity, Severity::Unknown);
        assert!(entry.service.is_none());
    }

    #[test]
    fn test_log_entry_builder() {
        let pane_id = Uuid::new_v4();
        let entry = LogEntry::new(pane_id, 1, Utc::now(), "Error occurred")
            .with_severity(Severity::Error)
            .with_service("api-service")
            .with_parsed_field("request_id", "abc123");

        assert_eq!(entry.severity, Severity::Error);
        assert_eq!(entry.service, Some("api-service".to_string()));
        assert_eq!(
            entry.parsed_fields.get("request_id"),
            Some(&"abc123".to_string())
        );
        assert!(entry.is_severe());
    }

    #[test]
    fn test_signature_generation() {
        let pane_id = Uuid::new_v4();
        let entry = LogEntry::new(pane_id, 1, Utc::now(), "  Error: Something failed  ")
            .with_severity(Severity::Error);

        let sig = entry.signature();
        assert!(sig.starts_with("ERROR:"));
        assert!(sig.contains("error: something failed"));
    }
}
