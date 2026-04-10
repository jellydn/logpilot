//! Integration tests for filter command

use logpilot::capture::tmux::TmuxCommand;
use logpilot::cli::filter::{detect_severity, line_matches, parse_severity};
use logpilot::models::Severity;
use regex::Regex;

/// Test: parse_severity converts strings correctly
#[test]
fn test_parse_severity_variations() {
    assert_eq!(parse_severity("trace"), Severity::Trace);
    assert_eq!(parse_severity("TRACE"), Severity::Trace);
    assert_eq!(parse_severity("debug"), Severity::Debug);
    assert_eq!(parse_severity("DEBUG"), Severity::Debug);
    assert_eq!(parse_severity("info"), Severity::Info);
    assert_eq!(parse_severity("INFO"), Severity::Info);
    assert_eq!(parse_severity("warn"), Severity::Warn);
    assert_eq!(parse_severity("WARN"), Severity::Warn);
    assert_eq!(parse_severity("warning"), Severity::Warn);
    assert_eq!(parse_severity("WARNING"), Severity::Warn);
    assert_eq!(parse_severity("error"), Severity::Error);
    assert_eq!(parse_severity("ERROR"), Severity::Error);
    assert_eq!(parse_severity("fatal"), Severity::Fatal);
    assert_eq!(parse_severity("FATAL"), Severity::Fatal);
    // Invalid defaults to Error
    assert_eq!(parse_severity("unknown"), Severity::Error);
    assert_eq!(parse_severity(""), Severity::Error);
}

/// Test: detect_severity identifies severity from log lines
#[test]
fn test_detect_severity_patterns() {
    // Fatal patterns
    assert_eq!(detect_severity("FATAL: process crashed"), Severity::Fatal);
    assert_eq!(detect_severity("panic: runtime error"), Severity::Fatal);
    assert_eq!(
        detect_severity("SIGSEGV: segmentation fault"),
        Severity::Fatal
    );
    assert_eq!(detect_severity("SIGKILL received"), Severity::Fatal);
    assert_eq!(detect_severity("CRASH: out of memory"), Severity::Fatal);

    // Error patterns
    assert_eq!(detect_severity("ERROR: connection failed"), Severity::Error);
    assert_eq!(detect_severity("Exception in thread main"), Severity::Error);
    assert_eq!(detect_severity("Failed to load config"), Severity::Error);
    assert_eq!(detect_severity("errno: -54"), Severity::Error);
    assert_eq!(
        detect_severity("ECONNREFUSED: connection refused"),
        Severity::Error
    );
    assert_eq!(detect_severity("read ECONNRESET"), Severity::Error);
    assert_eq!(
        detect_severity("EADDRNOTAVAIL: address not available"),
        Severity::Error
    );

    // Warning patterns
    assert_eq!(detect_severity("WARN: deprecated API"), Severity::Warn);
    assert_eq!(detect_severity("WARNING: low disk space"), Severity::Warn);
    assert_eq!(detect_severity("This is deprecated"), Severity::Warn);
    assert_eq!(detect_severity("CAUTION: hot surface"), Severity::Warn);

    // Info patterns
    assert_eq!(detect_severity("INFO: started server"), Severity::Info);
    assert_eq!(detect_severity("[INF] Server ready"), Severity::Info);

    // Debug patterns
    assert_eq!(detect_severity("DEBUG: variable x=42"), Severity::Debug);
    assert_eq!(detect_severity("[DBG] entering function"), Severity::Debug);
    assert_eq!(detect_severity("[DEBUG] query executed"), Severity::Debug);

    // Trace patterns
    assert_eq!(detect_severity("TRACE: entering method"), Severity::Trace);
    assert_eq!(detect_severity("[TRC] trace event"), Severity::Trace);

    // Unknown
    assert_eq!(detect_severity("random log line"), Severity::Unknown);
    assert_eq!(detect_severity("Hello world"), Severity::Unknown);
}

/// Test: line_matches with severity only
#[test]
fn test_line_matches_severity_only() {
    let no_pattern: Option<Regex> = None;

    // Error line meets error threshold
    assert!(line_matches("ERROR: something failed", Severity::Error, &no_pattern).is_some());

    // Debug line doesn't meet error threshold
    assert!(line_matches("DEBUG: test", Severity::Error, &no_pattern).is_none());

    // Warn meets warn threshold
    assert!(line_matches("WARN: caution", Severity::Warn, &no_pattern).is_some());

    // Error meets warn threshold (higher severity)
    assert!(line_matches("ERROR: critical", Severity::Warn, &no_pattern).is_some());
}

/// Test: line_matches with pattern regex
#[test]
fn test_line_matches_with_pattern() {
    let pattern = Some(Regex::new("database|postgres").unwrap());

    // Line matching pattern with sufficient severity
    let result = line_matches(
        "ERROR: database connection failed",
        Severity::Error,
        &pattern,
    );
    assert!(result.is_some());

    // Line matching pattern but below severity - pattern still triggers
    let result = line_matches("INFO: database query", Severity::Error, &pattern);
    assert!(result.is_some(), "Pattern should match even below severity");

    // Line not matching pattern
    let result = line_matches("ERROR: cache miss", Severity::Error, &pattern);
    assert!(result.is_none(), "Should not match pattern");

    // Line matching pattern with warn severity (below threshold)
    let result = line_matches("WARN: database slow", Severity::Error, &pattern);
    assert!(
        result.is_some(),
        "Pattern should match regardless of severity"
    );
}

/// Test: line_matches with complex regex
#[test]
fn test_line_matches_complex_patterns() {
    // Multiple patterns with OR
    let pattern = Some(Regex::new("(Error|Exception|Failed)").unwrap());
    assert!(line_matches("Error: timeout", Severity::Info, &pattern).is_some());
    assert!(line_matches("Exception: null pointer", Severity::Info, &pattern).is_some());
    assert!(line_matches("Failed to connect", Severity::Info, &pattern).is_some());

    // Case insensitive pattern
    let pattern = Some(Regex::new("(?i)database").unwrap());
    assert!(line_matches("DATABASE error", Severity::Info, &pattern).is_some());
    assert!(line_matches("Database connection", Severity::Info, &pattern).is_some());
    assert!(line_matches("db error", Severity::Info, &pattern).is_none());
}

/// Test: TmuxCommand::list_panes includes all panes from all windows
///
/// Note: This test requires a running tmux server with test session
#[tokio::test]
async fn test_list_panes_all_windows() {
    // Skip if tmux is not running
    if !TmuxCommand::is_installed() {
        println!("Skipping: tmux not installed");
        return;
    }

    // List all sessions
    let sessions = TmuxCommand::list_sessions().await.unwrap_or_default();

    for session in sessions {
        let panes = TmuxCommand::list_panes(&session).await;

        // Should succeed for valid sessions
        assert!(
            panes.is_ok(),
            "list_panes should succeed for session {}",
            session
        );

        let panes = panes.unwrap();

        // Count windows in session
        let windows_output = tokio::process::Command::new("tmux")
            .args(["list-windows", "-t", &session, "-F", "#I"])
            .output()
            .await;

        if let Ok(output) = windows_output {
            let window_count = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .count();

            println!(
                "Session {}: {} panes across {} windows",
                session,
                panes.len(),
                window_count
            );

            // Should have at least as many panes as windows
            assert!(
                panes.len() >= window_count,
                "Session {} should have panes >= windows (got {} panes, {} windows)",
                session,
                panes.len(),
                window_count
            );
        }

        // Verify pane IDs are valid (start with %)
        for pane in &panes {
            assert!(
                pane.starts_with('%'),
                "Pane ID should start with %: {}",
                pane
            );
        }
    }
}

/// Test: Severity ordering is correct
#[test]
fn test_severity_ordering() {
    // Lower severity values are less severe
    assert!(Severity::Trace < Severity::Debug);
    assert!(Severity::Debug < Severity::Info);
    assert!(Severity::Info < Severity::Warn);
    assert!(Severity::Warn < Severity::Error);
    assert!(Severity::Error < Severity::Fatal);

    // Same severity is equal
    assert!(Severity::Error == Severity::Error);
    assert!(Severity::Fatal == Severity::Fatal);
}

/// Test: filter command integration with various log formats
#[test]
fn test_detect_severity_real_log_formats() {
    // JSON structured logs
    assert_eq!(
        detect_severity(r#"{"level":"error","msg":"failed"}"#),
        Severity::Error
    );
    assert_eq!(
        detect_severity(r#"{"level":"warn","msg":"slow"}"#),
        Severity::Warn
    );

    // Common log prefixes
    assert_eq!(
        detect_severity("[ERROR] 2024-01-01: something failed"),
        Severity::Error
    );
    assert_eq!(
        detect_severity("[WARN] 2024-01-01: caution"),
        Severity::Warn
    );
    assert_eq!(
        detect_severity("[INFO] 2024-01-01: started"),
        Severity::Info
    );
    assert_eq!(
        detect_severity("[DEBUG] 2024-01-01: details"),
        Severity::Debug
    );
    // "crash" triggers Fatal
    assert_eq!(
        detect_severity("[ERROR] 2024-01-01: crash"),
        Severity::Fatal
    );

    // Node.js / JavaScript style
    assert_eq!(
        detect_severity("Error: Cannot find module 'foo'"),
        Severity::Error
    );
    assert_eq!(detect_severity("Warning: deprecated usage"), Severity::Warn);

    // Python style
    // Note: "Traceback" contains "trace" so it matches Trace severity
    assert_eq!(
        detect_severity("Traceback (most recent call last):"),
        Severity::Trace
    );
    assert_eq!(
        detect_severity("Exception: Something went wrong"),
        Severity::Error
    );

    // Rust style - "panic" triggers Fatal
    assert_eq!(
        detect_severity("thread 'main' panicked at 'oh no': src/main.rs:42"),
        Severity::Fatal
    );

    // Go style - "panic" triggers Fatal
    assert_eq!(
        detect_severity("panic: runtime error: index out of range"),
        Severity::Fatal
    );

    // Java style
    assert_eq!(
        detect_severity("java.lang.NullPointerException"),
        Severity::Error
    );
    assert_eq!(
        detect_severity("Caused by: java.net.ConnectException"),
        Severity::Error
    );
}
