//! Integration tests for User Story 2: Intelligent Log Analysis
//!
//! Tests pattern detection, deduplication, and incident clustering

use logpilot::models::{LogEntry, Pane, Pattern, Severity, Session};
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

/// Test fixture for creating test log entries
pub struct TestFixture {
    pub session_id: Uuid,
    pub pane_id: Uuid,
    sequence: u64,
}

impl TestFixture {
    pub fn new() -> Self {
        let session = Session::new("test-session");
        let pane = Pane::new(session.id, "test-pane:1.0");

        Self {
            session_id: session.id,
            pane_id: pane.id,
            sequence: 0,
        }
    }

    pub fn create_log_entry(&mut self, content: &str, severity: Severity) -> LogEntry {
        self.sequence += 1;
        LogEntry::new_with_severity(
            self.pane_id,
            self.sequence,
            chrono::Utc::now(),
            content.to_string(),
            severity,
        )
    }

    pub fn create_error_entry(&mut self, content: &str) -> LogEntry {
        self.create_log_entry(content, Severity::Error)
    }

    pub fn create_info_entry(&mut self, content: &str) -> LogEntry {
        self.create_log_entry(content, Severity::Info)
    }
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new()
    }
}

#[tokio::test]
async fn test_recurring_error_detection() {
    // Same error 5+ times in 60s window should trigger pattern detection
    let mut fixture = TestFixture::new();
    let error_content = "ERROR: Connection refused to database at localhost:5432";

    // Create 5 identical error entries within a short time
    let entries: Vec<LogEntry> = (0..5)
        .map(|_| fixture.create_error_entry(error_content))
        .collect();

    // TODO: Feed entries to PatternTracker and verify pattern is detected
    // when window_count >= 5

    // For now, verify entries have same content (preparation for dedup)
    assert!(entries.iter().all(|e| e.raw_content == error_content));
}

#[tokio::test]
async fn test_restart_loop_detection() {
    // Pattern: "starting service" -> "stopping service" -> "starting service"
    // within 30 seconds should trigger restart loop alert
    let mut fixture = TestFixture::new();

    let start_msg = "INFO: Starting checkout-service v1.2.3";
    let stop_msg = "INFO: Stopping checkout-service (SIGTERM received)";

    // Simulate restart loop sequence
    let entries = vec![
        fixture.create_info_entry(start_msg),
        fixture.create_info_entry(stop_msg),
        fixture.create_info_entry(start_msg), // Restart within 30s
    ];

    // TODO: Feed to RestartLoopDetector and verify detection
    assert_eq!(entries.len(), 3);
}

#[tokio::test]
async fn test_new_exception_detection() {
    // First-seen exception signature should be flagged
    let mut fixture = TestFixture::new();

    let new_exception = "Exception: NullPointerException at com.example.CheckoutController.processPayment";
    let entry = fixture.create_error_entry(new_exception);

    // TODO: Feed to NewExceptionDetector
    // Should be flagged as "new unseen exception"
    assert_eq!(entry.severity, Severity::Error);
}

#[tokio::test]
async fn test_deduplication_simhash() {
    // Similar stack traces should be deduplicated using SimHash
    let mut fixture = TestFixture::new();

    // Two very similar stack traces (only line numbers differ)
    let trace1 = "ERROR: java.lang.IllegalStateException: Connection closed
        at com.example.DBConnection.executeQuery(DBConnection.java:45)
        at com.example.UserService.getUser(UserService.java:23)
        at com.example.UserController.getUser(UserController.java:12)";

    let trace2 = "ERROR: java.lang.IllegalStateException: Connection closed
        at com.example.DBConnection.executeQuery(DBConnection.java:47)
        at com.example.UserService.getUser(UserService.java:25)
        at com.example.UserController.getUser(UserController.java:15)";

    let entry1 = fixture.create_error_entry(trace1);
    let entry2 = fixture.create_error_entry(trace2);

    // TODO: Run through deduplicator and verify they're grouped under same pattern
    // SimHash distance should be small for these similar traces
    assert_ne!(entry1.id, entry2.id);
}

#[tokio::test]
async fn test_pattern_sliding_window_decay() {
    // window_count should reset after 60s window passes
    let mut fixture = TestFixture::new();
    let error_content = "ERROR: TimeoutException";

    // Create 3 errors in quick succession
    let _entries: Vec<LogEntry> = (0..3)
        .map(|_| fixture.create_error_entry(error_content))
        .collect();

    // Wait for window to expire (60s)
    // In tests, we use a shorter window for practical reasons
    time::sleep(Duration::from_millis(100)).await;

    // TODO: Create new pattern tracker with 100ms window for testing
    // After window expires, new entry should create fresh window_count=1
}

#[tokio::test]
async fn test_error_rate_calculation() {
    // Calculate errors per minute for rate-based alerting
    let mut fixture = TestFixture::new();

    // Create 10 errors within 1 minute
    let errors: Vec<LogEntry> = (0..10)
        .map(|i| {
            let content = format!("ERROR: Request failed #{}" , i);
            fixture.create_error_entry(&content)
        })
        .collect();

    // TODO: Feed to ErrorRateCalculator
    // Should calculate ~10 errors/minute rate
    assert_eq!(errors.len(), 10);
}

#[tokio::test]
async fn test_incident_auto_creation() {
    // When pattern spike detected, incident should be auto-created
    let mut fixture = TestFixture::new();

    // Simulate a spike of errors
    let spike_entries: Vec<LogEntry> = (0..20)
        .map(|_| fixture.create_error_entry("ERROR: Database connection pool exhausted"))
        .collect();

    // TODO: Feed to IncidentDetector
    // Should create incident when spike threshold crossed
    assert_eq!(spike_entries.len(), 20);
}
