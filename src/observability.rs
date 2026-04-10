//! Structured logging and self-observability
//!
//! LogPilot practices "dogfooding" by logging its own metrics and events.
//! This module provides structured logging for internal operations.

use std::time::Instant;
use tracing::{debug, info, warn};

/// Metrics collected during LogPilot operation
#[derive(Debug, Default)]
pub struct Metrics {
    pub entries_captured: u64,
    pub entries_parsed: u64,
    pub entries_deduplicated: u64,
    pub patterns_detected: u64,
    pub incidents_created: u64,
    pub alerts_triggered: u64,
    pub start_time: Option<Instant>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            start_time: Some(Instant::now()),
            ..Default::default()
        }
    }

    /// Record a captured log entry
    pub fn record_entry_captured(&mut self) {
        self.entries_captured += 1;
        if self.entries_captured % 1000 == 0 {
            info!(
                entries_captured = self.entries_captured,
                "Captured 1000 entries"
            );
        }
    }

    /// Record a parsed entry
    pub fn record_entry_parsed(&mut self) {
        self.entries_parsed += 1;
    }

    /// Record a deduplicated entry
    pub fn record_entry_deduplicated(&mut self) {
        self.entries_deduplicated += 1;
    }

    /// Record a detected pattern
    pub fn record_pattern_detected(&mut self) {
        self.patterns_detected += 1;
        info!(
            pattern_count = self.patterns_detected,
            "New pattern detected"
        );
    }

    /// Record an incident creation
    pub fn record_incident_created(&mut self) {
        self.incidents_created += 1;
        warn!(incident_count = self.incidents_created, "Incident created");
    }

    /// Record an alert trigger
    pub fn record_alert_triggered(&mut self) {
        self.alerts_triggered += 1;
        warn!(alert_count = self.alerts_triggered, "Alert triggered");
    }

    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.map(|t| t.elapsed().as_secs()).unwrap_or(0)
    }

    /// Log current metrics summary
    pub fn log_summary(&self) {
        let uptime = self.uptime_seconds();
        let eps = if uptime > 0 {
            self.entries_captured as f64 / uptime as f64
        } else {
            0.0
        };

        info!(
            uptime_secs = uptime,
            entries_captured = self.entries_captured,
            entries_per_second = format!("{:.2}", eps),
            patterns_detected = self.patterns_detected,
            incidents_created = self.incidents_created,
            alerts_triggered = self.alerts_triggered,
            "LogPilot metrics summary"
        );
    }
}

/// Log structured event for capture operation
pub fn log_capture_event(session: &str, pane: &str, bytes: usize) {
    debug!(
        event = "capture",
        session = session,
        pane = pane,
        bytes = bytes,
        "Captured log data"
    );
}

/// Log structured event for parse operation
pub fn log_parse_event(severity: &str, service: Option<&str>, has_timestamp: bool) {
    debug!(
        event = "parse",
        severity = severity,
        service = service.unwrap_or("unknown"),
        has_timestamp = has_timestamp,
        "Parsed log entry"
    );
}

/// Log structured event for alert evaluation
pub fn log_alert_evaluation(alert_type: &str, triggered: bool, threshold: f64, current: f64) {
    if triggered {
        warn!(
            event = "alert_triggered",
            alert_type = alert_type,
            threshold = threshold,
            current_value = current,
            "Alert threshold exceeded"
        );
    } else {
        debug!(
            event = "alert_evaluated",
            alert_type = alert_type,
            threshold = threshold,
            current_value = current,
            "Alert below threshold"
        );
    }
}

/// Log structured event for MCP request
pub fn log_mcp_request(method: &str, resource_uri: Option<&str>) {
    info!(
        event = "mcp_request",
        method = method,
        resource = resource_uri.unwrap_or("-"),
        "MCP request received"
    );
}

/// Log structured event for session state change
pub fn log_session_state(session: &str, old_state: &str, new_state: &str) {
    info!(
        event = "session_state_change",
        session = session,
        old_state = old_state,
        new_state = new_state,
        "Session state changed"
    );
}

/// Log buffer statistics
pub fn log_buffer_stats(entries: usize, capacity: usize, utilization_percent: f64) {
    debug!(
        event = "buffer_stats",
        entries = entries,
        capacity = capacity,
        utilization_pct = format!("{:.1}", utilization_percent),
        "Buffer status"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        let mut metrics = Metrics::new();

        metrics.record_entry_captured();
        metrics.record_entry_parsed();
        metrics.record_pattern_detected();

        assert_eq!(metrics.entries_captured, 1);
        assert_eq!(metrics.entries_parsed, 1);
        assert_eq!(metrics.patterns_detected, 1);
    }

    #[test]
    fn test_metrics_uptime() {
        let metrics = Metrics::new();
        // Uptime should be very small (just created)
        assert!(metrics.uptime_seconds() < 1);
    }

    #[test]
    fn test_log_capture_event() {
        // Just verify it doesn't panic
        log_capture_event("test-session", "pane-1", 1024);
    }

    #[test]
    fn test_log_parse_event() {
        log_parse_event("ERROR", Some("api-service"), true);
        log_parse_event("INFO", None, false);
    }

    #[test]
    fn test_log_alert_evaluation() {
        log_alert_evaluation("ErrorRate", true, 10.0, 15.0);
        log_alert_evaluation("ErrorRate", false, 10.0, 5.0);
    }
}
