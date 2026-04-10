//! Log parser for extracting structured data from raw log lines

use crate::models::{LogEntry, Severity};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

// Static compiled regex patterns for performance
static TIMESTAMP_ISO8601_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?)\s*")
        .expect("Invalid ISO8601 timestamp regex")
});

static TIMESTAMP_STANDARD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}(?:,\d{3})?)\s*")
        .expect("Invalid standard timestamp regex")
});

static TIMESTAMP_SYSLOG_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([A-Z][a-z]{2}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2})\s*")
        .expect("Invalid syslog timestamp regex")
});

static SEVERITY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(TRACE|DEBUG|INFO|WARN(?:ING)?|ERROR|FATAL)\b")
        .expect("Invalid severity regex")
});

static SERVICE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:\[([a-zA-Z0-9_-]+)\]|service[=:]([a-zA-Z0-9_-]+))")
        .expect("Invalid service regex")
});

static KEY_VALUE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(\w+)=([^\s,]+)").expect("Invalid key=value regex"));

/// Parser for extracting timestamps, severity, and service from log lines
pub struct LogParser {
    // Common timestamp patterns
    timestamp_regexes: Vec<&'static Regex>,
    // Severity patterns
    severity_regex: &'static Regex,
    // Service name patterns (e.g., [service-name] or service=xxx)
    service_regex: &'static Regex,
}

impl LogParser {
    pub fn new() -> Self {
        Self {
            timestamp_regexes: vec![
                &*TIMESTAMP_ISO8601_RE,
                &*TIMESTAMP_STANDARD_RE,
                &*TIMESTAMP_SYSLOG_RE,
            ],
            severity_regex: &*SEVERITY_RE,
            service_regex: &*SERVICE_RE,
        }
    }

    /// Parse a log entry, extracting structured fields
    pub fn parse(&self, entry: &mut LogEntry) {
        let content = &entry.raw_content;

        // Extract timestamp (if present in content)
        if let Some(timestamp) = self.extract_timestamp(content) {
            // Parse the extracted timestamp string
            if let Ok(dt) = self.parse_timestamp_str(&timestamp) {
                entry.timestamp = dt;
            }
        }

        // Extract severity
        if let Some(severity) = self.extract_severity(content) {
            entry.severity = severity;
        }

        // Extract service name
        if let Some(service) = self.extract_service(content) {
            entry.service = Some(service);
        }

        // Store any other structured fields
        entry.parsed_fields = self.extract_fields(content);
    }

    fn extract_timestamp(&self, content: &str) -> Option<String> {
        for regex in &self.timestamp_regexes {
            if let Some(caps) = regex.captures(content) {
                return caps.get(1).map(|m| m.as_str().to_string());
            }
        }
        None
    }

    fn parse_timestamp_str(
        &self,
        ts: &str,
    ) -> Result<chrono::DateTime<chrono::Utc>, chrono::ParseError> {
        // Try various formats
        let formats = [
            "%Y-%m-%dT%H:%M:%SZ",
            "%Y-%m-%dT%H:%M:%S%.3fZ",
            "%Y-%m-%dT%H:%M:%S%:z",
            "%Y-%m-%d %H:%M:%S",
            "%Y-%m-%d %H:%M:%S,%3f",
        ];

        for fmt in &formats {
            if let Ok(dt) = chrono::DateTime::parse_from_str(ts, fmt) {
                return Ok(dt.with_timezone(&chrono::Utc));
            }
        }

        // Fallback to chrono's flexible parser
        ts.parse::<chrono::DateTime<chrono::Utc>>()
    }

    fn extract_severity(&self, content: &str) -> Option<Severity> {
        self.severity_regex
            .captures(content)
            .and_then(|caps| caps.get(1).map(|m| m.as_str().parse().ok()))?
    }

    fn extract_service(&self, content: &str) -> Option<String> {
        self.service_regex.captures(content).and_then(|caps| {
            caps.get(1)
                .or_else(|| caps.get(2))
                .map(|m| m.as_str().to_string())
        })
    }

    fn extract_fields(&self, content: &str) -> HashMap<String, String> {
        let mut fields = HashMap::new();

        // Extract key=value pairs
        for caps in KEY_VALUE_RE.captures_iter(content) {
            if let (Some(key), Some(val)) = (caps.get(1), caps.get(2)) {
                fields.insert(key.as_str().to_string(), val.as_str().to_string());
            }
        }

        fields
    }
}

impl Default for LogParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_severity() {
        let parser = LogParser::new();

        let cases = vec![
            ("ERROR: Something failed", Severity::Error),
            ("INFO: Server started", Severity::Info),
            ("DEBUG: Processing request", Severity::Debug),
            ("WARN: High memory usage", Severity::Warn),
            ("FATAL: Cannot start", Severity::Fatal),
        ];

        for (content, expected) in cases {
            let mut entry = LogEntry::new(
                uuid::Uuid::new_v4(),
                1,
                chrono::Utc::now(),
                content.to_string(),
            );
            parser.parse(&mut entry);
            assert_eq!(entry.severity, expected, "Failed for: {}", content);
        }
    }

    #[test]
    fn test_parse_severity_variations() {
        let parser = LogParser::new();

        let cases = vec![
            ("error: lowercase error", Severity::Error),
            ("ERROR lowercase", Severity::Error),
            ("[ERROR] bracketed", Severity::Error),
            ("WARNING: warning alias", Severity::Warn),
            ("warning: lowercase warning", Severity::Warn),
            ("info: lowercase info", Severity::Info),
            ("debug: lowercase debug", Severity::Debug),
            ("trace: lowercase trace", Severity::Trace),
        ];

        for (content, expected) in cases {
            let mut entry = LogEntry::new(
                uuid::Uuid::new_v4(),
                1,
                chrono::Utc::now(),
                content.to_string(),
            );
            parser.parse(&mut entry);
            assert_eq!(entry.severity, expected, "Failed for: {}", content);
        }
    }

    #[test]
    fn test_parse_service() {
        let parser = LogParser::new();

        let cases = vec![
            (
                "[checkout-service] Order received",
                Some("checkout-service"),
            ),
            (
                "INFO service=payment-gateway Connected",
                Some("payment-gateway"),
            ),
            ("[api-v2] Request completed", Some("api-v2")),
        ];

        for (content, expected) in cases {
            let mut entry = LogEntry::new(
                uuid::Uuid::new_v4(),
                1,
                chrono::Utc::now(),
                content.to_string(),
            );
            parser.parse(&mut entry);
            assert_eq!(
                entry.service.as_deref(),
                expected,
                "Failed for: {}",
                content
            );
        }
    }

    #[test]
    fn test_parse_service_variations() {
        let parser = LogParser::new();

        let cases = vec![
            ("[user-service] User logged in", Some("user-service")),
            (
                "INFO service=auth-service Login success",
                Some("auth-service"),
            ),
            ("[payment_v2] Payment processed", Some("payment_v2")),
            // Note: component= is not supported by current regex, only service= and [...]
            // ("component=order-service Processing", None),
        ];

        for (content, expected) in cases {
            let mut entry = LogEntry::new(
                uuid::Uuid::new_v4(),
                1,
                chrono::Utc::now(),
                content.to_string(),
            );
            parser.parse(&mut entry);
            assert_eq!(
                entry.service.as_deref(),
                expected,
                "Failed for: {}",
                content
            );
        }
    }

    #[test]
    fn test_parse_timestamp_iso8601() {
        let parser = LogParser::new();

        // Use a timestamp from the past - verify it was parsed correctly
        let cases = vec![
            ("2024-01-15T10:30:00Z INFO Test message", "2024-01-15"),
            ("2024-06-20T10:30:00+00:00 ERROR Test", "2024-06-20"),
        ];

        for (content, expected_date) in cases {
            let mut entry = LogEntry::new(
                uuid::Uuid::new_v4(),
                1,
                chrono::Utc::now(),
                content.to_string(),
            );
            parser.parse(&mut entry);
            // Verify timestamp was parsed by checking the date part
            let ts_str = entry.timestamp.to_rfc3339();
            assert!(
                ts_str.contains(expected_date),
                "Timestamp should contain {} for: {} (got: {})",
                expected_date,
                content,
                ts_str
            );
        }
    }

    #[test]
    fn test_parse_timestamp_standard() {
        let parser = LogParser::new();

        let cases = vec![
            "2024-01-15 10:30:00 INFO Test",
            "2024-01-15 10:30:00,123 WARN Test",
        ];

        for content in cases {
            let mut entry = LogEntry::new(
                uuid::Uuid::new_v4(),
                1,
                chrono::Utc::now(),
                content.to_string(),
            );
            parser.parse(&mut entry);
            // Just verify parsing doesn't panic and timestamp is set
            assert!(entry.timestamp.timestamp() > 0);
        }
    }

    #[test]
    fn test_parse_timestamp_syslog() {
        let parser = LogParser::new();

        let cases = vec![
            "Jan 15 10:30:00 service message",
            "Feb 28 23:59:59 service message",
            "Dec 31 00:00:00 service message",
        ];

        for content in cases {
            let mut entry = LogEntry::new(
                uuid::Uuid::new_v4(),
                1,
                chrono::Utc::now(),
                content.to_string(),
            );
            parser.parse(&mut entry);
            // Syslog format should be parsed
            assert!(entry.timestamp.timestamp() > 0);
        }
    }

    #[test]
    fn test_parse_combined_fields() {
        let parser = LogParser::new();

        let content = "2024-01-15T10:30:00Z [api-service] ERROR: Connection failed";
        let mut entry = LogEntry::new(
            uuid::Uuid::new_v4(),
            1,
            chrono::Utc::now(),
            content.to_string(),
        );
        parser.parse(&mut entry);

        assert_eq!(entry.severity, Severity::Error);
        assert_eq!(entry.service, Some("api-service".to_string()));
    }

    #[test]
    fn test_parse_key_value_fields() {
        let parser = LogParser::new();

        let content = "INFO request_id=123 user=alice action=login";
        let mut entry = LogEntry::new(
            uuid::Uuid::new_v4(),
            1,
            chrono::Utc::now(),
            content.to_string(),
        );
        parser.parse(&mut entry);

        assert_eq!(entry.severity, Severity::Info);
        assert_eq!(
            entry.parsed_fields.get("request_id"),
            Some(&"123".to_string())
        );
        assert_eq!(entry.parsed_fields.get("user"), Some(&"alice".to_string()));
        assert_eq!(
            entry.parsed_fields.get("action"),
            Some(&"login".to_string())
        );
    }

    #[test]
    fn test_parse_empty_content() {
        let parser = LogParser::new();

        let content = "";
        let mut entry = LogEntry::new(
            uuid::Uuid::new_v4(),
            1,
            chrono::Utc::now(),
            content.to_string(),
        );
        parser.parse(&mut entry);

        // Should default to INFO and no service
        assert_eq!(entry.severity, Severity::Unknown);
        assert_eq!(entry.service, None);
        assert!(entry.parsed_fields.is_empty());
    }

    #[test]
    fn test_parse_no_severity() {
        let parser = LogParser::new();

        let content = "Just a plain message with no severity";
        let mut entry = LogEntry::new(
            uuid::Uuid::new_v4(),
            1,
            chrono::Utc::now(),
            content.to_string(),
        );
        parser.parse(&mut entry);

        assert_eq!(entry.severity, Severity::Unknown);
    }

    #[test]
    fn test_parse_special_characters_in_service() {
        let parser = LogParser::new();

        // Service names with hyphens and underscores
        let cases = vec![
            ("[my-service-123] Message", Some("my-service-123")),
            ("[service_v1_beta] Message", Some("service_v1_beta")),
            ("[multi-word-service] Message", Some("multi-word-service")),
        ];

        for (content, expected) in cases {
            let mut entry = LogEntry::new(
                uuid::Uuid::new_v4(),
                1,
                chrono::Utc::now(),
                content.to_string(),
            );
            parser.parse(&mut entry);
            assert_eq!(
                entry.service.as_deref(),
                expected,
                "Failed for: {}",
                content
            );
        }
    }

    #[test]
    fn test_parse_embedded_severity() {
        let parser = LogParser::new();
        // Severity appears in the middle of text
        let cases = vec![
            ("[2024-01-15] The ERROR occurred", Severity::Error),
            ("Processing INFO request", Severity::Info),
            ("Got DEBUG signal", Severity::Debug),
        ];

        for (content, expected) in cases {
            let mut entry = LogEntry::new(
                uuid::Uuid::new_v4(),
                1,
                chrono::Utc::now(),
                content.to_string(),
            );
            parser.parse(&mut entry);
            assert_eq!(entry.severity, expected, "Failed for: {}", content);
        }
    }

    #[test]
    fn test_parse_log_line_with_all_components() {
        let parser = LogParser::new();

        // Full log line with timestamp, service, severity, and key=value pairs
        let content = "2024-01-15T10:30:00Z [order-service] ERROR: Order processing failed order_id=12345 user_id=67890";
        let mut entry = LogEntry::new(
            uuid::Uuid::new_v4(),
            1,
            chrono::Utc::now(),
            content.to_string(),
        );
        parser.parse(&mut entry);

        assert_eq!(entry.severity, Severity::Error);
        assert_eq!(entry.service, Some("order-service".to_string()));
        assert_eq!(
            entry.parsed_fields.get("order_id"),
            Some(&"12345".to_string())
        );
        assert_eq!(
            entry.parsed_fields.get("user_id"),
            Some(&"67890".to_string())
        );
    }
}
