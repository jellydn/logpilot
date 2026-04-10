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
}
