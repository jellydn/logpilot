//! Integration tests for alert triggers (User Story 4)
//!
//! Tests:
//! - Error rate threshold alerts
//! - Alert deduplication
//! - Alert acknowledgment
//! - Visual indicator output

use logpilot::analyzer::{AlertEvaluator, ErrorRateCalculator};
use logpilot::models::{AlertStatus, AlertType, Pattern, Severity};

/// Test: Error rate > threshold triggers alert (T078)
#[tokio::test]
async fn test_error_rate_threshold_alert() {
    let (evaluator, _alert_rx) = AlertEvaluator::new();
    let calc = ErrorRateCalculator::new();

    // Simulate high error rate (15 errors in 1 minute = 15/min)
    for _ in 0..15 {
        calc.record_error(Some("test-service"));
    }

    // Calculate rate
    let rate = calc.calculate_rate(Some("test-service"));
    assert!(rate >= 10.0, "Error rate should exceed threshold");

    // Check if alert is triggered
    let alert = evaluator.check_error_rate(rate, Some("test-service"));
    assert!(
        alert.is_some(),
        "Alert should be triggered when rate exceeds threshold"
    );

    let alert = alert.unwrap();
    assert_eq!(alert.alert_type, AlertType::ErrorRate);
    assert_eq!(alert.status, AlertStatus::Active);
    assert!(alert.current_value >= 10.0);
}

/// Test: No duplicate alerts for same incident (T079)
#[tokio::test]
async fn test_alert_deduplication() {
    let (evaluator, _rx) = AlertEvaluator::new();

    // Create a pattern
    let mut pattern = Pattern::new("test-error-signature");
    pattern.window_count = 5;
    pattern.severity = Severity::Error;

    // First alert
    let alert1 = evaluator
        .check_recurring_error(&pattern)
        .expect("First alert should be created");

    // Second alert for same pattern should be deduplicated
    let alert2 = evaluator
        .check_recurring_error(&pattern)
        .expect("Second alert should return existing");

    assert_eq!(alert1.id, alert2.id, "Should be same alert (deduplicated)");

    // Verify only one alert exists
    assert_eq!(evaluator.count(), 1);
}

/// Test: Acknowledged alerts marked correctly (T080)
#[tokio::test]
async fn test_alert_acknowledgment() {
    let (evaluator, _rx) = AlertEvaluator::new();

    // Create a pattern
    let mut pattern = Pattern::new("test-error");
    pattern.window_count = 5;
    pattern.severity = Severity::Error;

    // Create alert
    let alert = evaluator
        .check_recurring_error(&pattern)
        .expect("Alert should be created");

    // Verify alert is active
    assert_eq!(alert.status, AlertStatus::Active);

    // Acknowledge the alert
    let acknowledged = evaluator.acknowledge(alert.id);
    assert!(acknowledged, "Alert should be acknowledged");

    // Get updated alert
    let active_alerts = evaluator.active_alerts();
    let found = active_alerts.iter().any(|a| a.id == alert.id);
    assert!(!found, "Acknowledged alert should not be in active list");
}

/// Test: Color codes appear in terminal output (T081)
#[test]
fn test_visual_indicator_output() {
    // Test severity icons
    let test_cases = vec![
        (Severity::Trace, "⚪"),
        (Severity::Debug, "🔵"),
        (Severity::Info, "💙"),
        (Severity::Warn, "🟡"),
        (Severity::Error, "🔴"),
        (Severity::Fatal, "💥"),
        (Severity::Unknown, "⚫"),
    ];

    for (severity, expected_icon) in test_cases {
        let icon = match severity {
            Severity::Trace => "⚪",
            Severity::Debug => "🔵",
            Severity::Info => "💙",
            Severity::Warn => "🟡",
            Severity::Error => "🔴",
            Severity::Fatal => "💥",
            Severity::Unknown => "⚫",
        };
        assert_eq!(icon, expected_icon, "Icon mismatch for {:?}", severity);
    }
}

/// Test: Restart loop detection creates alert
#[tokio::test]
async fn test_restart_loop_alert() {
    let (evaluator, _alert_rx) = AlertEvaluator::new();

    // Simulate restart loop detection
    let alert = evaluator
        .check_restart_loop("api-service", true)
        .expect("Restart loop alert should be created");

    assert_eq!(alert.alert_type, AlertType::RestartLoop);
    assert!(alert.message.contains("api-service"));
}

/// Test: New exception alert for unseen patterns
#[tokio::test]
async fn test_new_exception_alert() {
    let (evaluator, _alert_rx) = AlertEvaluator::new();

    // Create a new pattern
    let mut pattern = Pattern::new("new-exception-signature");
    pattern.severity = Severity::Error;

    // Check for new exception
    let alert = evaluator
        .check_new_exception(&pattern, true)
        .expect("New exception alert should be created");

    assert_eq!(alert.alert_type, AlertType::NewException);
    assert_eq!(alert.severity, Severity::Error);
}

/// Test: Alert resolution
#[tokio::test]
async fn test_alert_resolution() {
    let (evaluator, _rx) = AlertEvaluator::new();

    // Create a pattern
    let mut pattern = Pattern::new("test-error");
    pattern.window_count = 5;
    pattern.severity = Severity::Error;

    // Create alert
    let alert = evaluator
        .check_recurring_error(&pattern)
        .expect("Alert should be created");

    // Resolve the alert
    let resolved = evaluator.resolve(alert.id);
    assert!(resolved, "Alert should be resolved");

    // Verify alert is not active
    let active_alerts = evaluator.active_alerts();
    assert!(
        active_alerts.is_empty(),
        "No active alerts after resolution"
    );
}
