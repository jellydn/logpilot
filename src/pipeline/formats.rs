//! Structured log format parsers (JSON, logfmt)

use crate::models::LogEntry;
use serde_json::Value;
use std::collections::HashMap;

/// Parser for structured log formats
pub struct FormatParser;

impl FormatParser {
    /// Try to parse as JSON log format
    pub fn try_parse_json(entry: &mut LogEntry) -> bool {
        if let Ok(json) = serde_json::from_str::<Value>(&entry.raw_content) {
            // Extract fields from JSON
            if let Some(obj) = json.as_object() {
                // Extract timestamp
                if let Some(ts) = obj
                    .get("timestamp")
                    .or_else(|| obj.get("time"))
                    .or_else(|| obj.get("ts"))
                    .and_then(|v| v.as_str())
                {
                    if let Ok(dt) = ts.parse::<chrono::DateTime<chrono::Utc>>() {
                        entry.timestamp = dt;
                    }
                }

                // Extract severity/level
                if let Some(level) = obj
                    .get("level")
                    .or_else(|| obj.get("severity"))
                    .or_else(|| obj.get("log_level"))
                    .and_then(|v| v.as_str())
                {
                    if let Ok(sev) = level.parse() {
                        entry.severity = sev;
                    }
                }

                // Extract service/component
                if let Some(service) = obj
                    .get("service")
                    .or_else(|| obj.get("component"))
                    .or_else(|| obj.get("logger"))
                    .and_then(|v| v.as_str())
                {
                    entry.service = Some(service.to_string());
                }

                // Extract message
                if let Some(msg) = obj
                    .get("message")
                    .or_else(|| obj.get("msg"))
                    .or_else(|| obj.get("log"))
                    .and_then(|v| v.as_str())
                {
                    // Add message to parsed fields
                    entry
                        .parsed_fields
                        .insert("message".to_string(), msg.to_string());
                }

                // Store all other fields
                for (key, value) in obj {
                    if key != "timestamp"
                        && key != "time"
                        && key != "level"
                        && key != "severity"
                        && key != "service"
                        && key != "component"
                        && key != "message"
                        && key != "msg"
                    {
                        let val_str = match value {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        entry.parsed_fields.insert(key.clone(), val_str);
                    }
                }

                return true;
            }
        }
        false
    }

    /// Try to parse as logfmt (key=value pairs)
    pub fn try_parse_logfmt(entry: &mut LogEntry) -> bool {
        let mut fields = HashMap::new();
        let mut found_structured = false;

        // Parse logfmt: key=value key="value with spaces"
        let content = &entry.raw_content;

        // Simple logfmt parser
        let mut remaining = content.as_str();
        while let Some(pos) = remaining.find('=') {
            found_structured = true;

            // Extract key (word before =)
            let key_start = remaining[..pos]
                .rfind(|c: char| c.is_whitespace())
                .map(|i| i + 1)
                .unwrap_or(0);
            let key = remaining[key_start..pos].trim();

            // Extract value
            remaining = &remaining[pos + 1..];
            let (value, rest) = Self::extract_logfmt_value(remaining);
            remaining = rest;

            fields.insert(key.to_string(), value);
        }

        if found_structured {
            // Map common logfmt fields
            if let Some(level) = fields.get("level").or_else(|| fields.get("lvl")) {
                if let Ok(sev) = level.parse() {
                    entry.severity = sev;
                }
            }
            if let Some(ts) = fields.get("ts").or_else(|| fields.get("timestamp")) {
                if let Ok(dt) = ts.parse::<chrono::DateTime<chrono::Utc>>() {
                    entry.timestamp = dt;
                }
            }
            if let Some(service) = fields.get("service").or_else(|| fields.get("svc")) {
                entry.service = Some(service.clone());
            }

            entry.parsed_fields.extend(fields);
        }

        found_structured
    }

    fn extract_logfmt_value(input: &str) -> (String, &str) {
        let input = input.trim_start();

        if let Some(rest) = input.strip_prefix('"') {
            // Quoted string
            let mut escaped = false;
            let mut value = String::new();

            for (i, c) in rest.chars().enumerate() {
                if escaped {
                    value.push(c);
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    let remaining = &rest[i + 1..];
                    return (value, remaining);
                } else {
                    value.push(c);
                }
            }
            // Unterminated quote, take rest
            (value, "")
        } else {
            // Unquoted value (until space)
            if let Some(pos) = input.find(|c: char| c.is_whitespace()) {
                (input[..pos].to_string(), &input[pos..])
            } else {
                (input.to_string(), "")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Severity;

    #[test]
    fn test_parse_json_log() {
        let json_line = r#"{"timestamp":"2024-01-15T10:30:00Z","level":"ERROR","service":"api","message":"Connection failed"}"#;

        let mut entry = LogEntry::new(
            uuid::Uuid::new_v4(),
            1,
            chrono::Utc::now(),
            json_line.to_string(),
        );

        assert!(FormatParser::try_parse_json(&mut entry));
        assert_eq!(entry.severity, Severity::Error);
        assert_eq!(entry.service, Some("api".to_string()));
    }

    #[test]
    fn test_parse_logfmt() {
        let logfmt_line = "ts=2024-01-15T10:30:00Z level=WARN service=payment msg=\"high latency\"";

        let mut entry = LogEntry::new(
            uuid::Uuid::new_v4(),
            1,
            chrono::Utc::now(),
            logfmt_line.to_string(),
        );

        assert!(FormatParser::try_parse_logfmt(&mut entry));
        assert_eq!(entry.severity, Severity::Warn);
        assert_eq!(entry.service, Some("payment".to_string()));
    }
}
