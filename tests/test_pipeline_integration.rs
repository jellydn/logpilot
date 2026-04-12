//! Integration tests for the capture → pipeline → buffer → MCP flow.
//!
//! Covers:
//! 1. Buffer ingestion + severity filtering (only ERROR/FATAL reach SQLite)
//! 2. MCP resource reads via SessionDataStore + ResourceHandler
//! 3. Dedup pipeline (duplicate log lines deduplicated)
//! 4. Severity ordering (get_entries_by_severity returns correct subsets)

use chrono::Utc;
use logpilot::{
    buffer::manager::BufferManager,
    mcp::{data_store::SessionDataStore, resources::ResourceHandler},
    models::{LogEntry, Severity},
    pipeline::{
        dedup::{generate_signature, Deduplicator},
        parser::LogParser,
    },
};
use std::collections::HashMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_entry(pane_id: Uuid, seq: u64, content: &str) -> LogEntry {
    LogEntry {
        id: Uuid::new_v4(),
        pane_id,
        sequence: seq,
        timestamp: Utc::now(),
        severity: Severity::Unknown,
        service: None,
        raw_content: content.to_string(),
        parsed_fields: HashMap::new(),
        received_at: Utc::now(),
    }
}

fn make_entry_with_severity(pane_id: Uuid, seq: u64, content: &str, sev: Severity) -> LogEntry {
    let mut e = make_entry(pane_id, seq, content);
    e.severity = sev;
    e
}

// ---------------------------------------------------------------------------
// 1. Buffer ingestion + severity filtering
// ---------------------------------------------------------------------------

/// Feed log lines with mixed severities through `BufferManager::with_persistence`
/// and verify that only ERROR / FATAL entries are written to SQLite.
#[tokio::test]
async fn test_buffer_only_persists_error_and_fatal() {
    let manager = BufferManager::with_persistence(
        ":memory:",
        /*capacity=*/ 1000,
        /*retention_minutes=*/ 60,
        /*persist_severity=*/ Severity::Error,
    )
    .await
    .expect("should create in-memory persistence store");

    let pane_id = Uuid::new_v4();
    manager.create_buffer(pane_id).await;

    // Ingest one entry per severity level
    let entries = vec![
        make_entry_with_severity(pane_id, 1, "TRACE message", Severity::Trace),
        make_entry_with_severity(pane_id, 2, "DEBUG message", Severity::Debug),
        make_entry_with_severity(pane_id, 3, "INFO message", Severity::Info),
        make_entry_with_severity(pane_id, 4, "WARN message", Severity::Warn),
        make_entry_with_severity(pane_id, 5, "ERROR message", Severity::Error),
        make_entry_with_severity(pane_id, 6, "FATAL message", Severity::Fatal),
    ];

    for entry in entries {
        manager.add_entry(entry).await.expect("add_entry should succeed");
    }

    // All six entries should be in the ring buffer
    let in_memory = manager.get_entries(pane_id).await;
    assert_eq!(in_memory.len(), 6, "ring buffer should hold all 6 entries");

    // Only ERROR and FATAL should be persisted to SQLite
    let now = Utc::now();
    let since = now - chrono::Duration::hours(1);
    let persisted = manager
        .query_persisted(since, now, None)
        .await
        .expect("query_persisted should succeed");

    assert_eq!(
        persisted.len(),
        2,
        "only ERROR and FATAL should be persisted, got: {:?}",
        persisted.iter().map(|e| &e.severity).collect::<Vec<_>>()
    );
    assert!(
        persisted.iter().all(|e| e.severity == Severity::Error || e.severity == Severity::Fatal),
        "persisted entries must be ERROR or FATAL"
    );
}

// ---------------------------------------------------------------------------
// 2. MCP resource reads
// ---------------------------------------------------------------------------

/// Ingest entries into a `SessionDataStore`, then read back content via
/// `ResourceHandler::build_entries` and assert it contains the expected data.
/// This mirrors how the MCP server would serve `logpilot://session/{name}/entries`.
#[tokio::test]
async fn test_mcp_resource_read_session_entries() {
    let store = SessionDataStore::new();
    let session = "test-session";
    store.create_session(session).await;

    // Ingest entries directly into the store (simulates watch pipeline output)
    let pane_id = Uuid::new_v4();
    let error_entry = make_entry_with_severity(pane_id, 1, "ERROR: disk full", Severity::Error);
    let info_entry = make_entry_with_severity(pane_id, 2, "INFO: heartbeat ok", Severity::Info);

    store.add_entry(session, error_entry.clone()).await;
    store.add_entry(session, info_entry.clone()).await;

    // Retrieve session data and build the MCP resource content
    let data = store.get_session(session).await.expect("session should exist");

    let content =
        ResourceHandler::build_entries(session, &data.entries, &HashMap::new());

    // The response URI should reference the session
    assert!(
        content.uri.contains(session),
        "resource URI should contain session name"
    );
    assert_eq!(
        content.mime_type,
        Some("application/json".to_string()),
        "mime type should be application/json"
    );

    // Parse JSON and assert on structured fields rather than raw text
    let entries_json: serde_json::Value =
        serde_json::from_str(&content.text).expect("entries payload should be valid JSON");
    let entries = entries_json["entries"]
        .as_array()
        .expect("entries should be an array");
    assert!(
        entries
            .iter()
            .any(|e| e["raw_content"] == "ERROR: disk full"),
        "entries resource should contain the error message"
    );
    assert!(
        entries
            .iter()
            .any(|e| e["raw_content"] == "INFO: heartbeat ok"),
        "entries resource should contain the info message"
    );
}

/// Verify that the `logpilot://session/{name}/summary` resource accurately
/// reflects the number and severity breakdown of ingested entries.
#[tokio::test]
async fn test_mcp_resource_read_session_summary() {
    let store = SessionDataStore::new();
    let session = "summary-session";
    store.create_session(session).await;

    let pane_id = Uuid::new_v4();
    store
        .add_entry(session, make_entry_with_severity(pane_id, 1, "ERROR: boom", Severity::Error))
        .await;
    store
        .add_entry(session, make_entry_with_severity(pane_id, 2, "INFO: ok", Severity::Info))
        .await;
    store
        .add_entry(session, make_entry_with_severity(pane_id, 3, "FATAL: crash", Severity::Fatal))
        .await;

    let data = store.get_session(session).await.expect("session should exist");
    let now = Utc::now();
    let summary = ResourceHandler::build_summary(
        session,
        &data.entries,
        &[],
        &[],
        &[],
        now - chrono::Duration::minutes(30),
        now,
    );

    // Parse the JSON payload and assert on structured fields
    let summary_json: serde_json::Value =
        serde_json::from_str(&summary.text).expect("summary payload should be valid JSON");
    assert_eq!(
        summary_json["session_name"].as_str(),
        Some(session),
        "summary should contain session name"
    );
    assert_eq!(
        summary_json["total_entries"].as_u64(),
        Some(3),
        "summary should report total_entries: 3, got: {}",
        summary.text
    );
}

// ---------------------------------------------------------------------------
// 3. Dedup pipeline
// ---------------------------------------------------------------------------

/// Ingest two nearly-identical log lines; the second should be identified as a
/// duplicate by the `Deduplicator` (SimHash distance within threshold).
#[tokio::test]
async fn test_dedup_pipeline_identifies_duplicates() {
    let mut dedup = Deduplicator::new();
    let pane_id = Uuid::new_v4();

    let first = make_entry(pane_id, 1, "ERROR: connection refused to db at localhost:5432");
    let second = make_entry(pane_id, 2, "ERROR: connection refused to db at localhost:5432");

    // The first entry should not be a duplicate
    assert!(
        dedup.find_duplicate(&first).is_none(),
        "first entry should not be a duplicate"
    );

    // Register the first entry
    let sig = generate_signature(&first.raw_content);
    dedup.add_signature(&first, sig);

    // The second (identical) entry should now be detected as a duplicate
    assert!(
        dedup.find_duplicate(&second).is_some(),
        "identical second entry should be flagged as a duplicate"
    );

    // A clearly different entry should NOT match
    let different = make_entry(pane_id, 3, "INFO: server started successfully on port 8080");
    assert!(
        dedup.find_duplicate(&different).is_none(),
        "a different log line should not match"
    );
}

/// Verify that the `Deduplicator` also catches near-duplicate lines that differ
/// only in variable parts such as line numbers.
#[tokio::test]
async fn test_dedup_pipeline_similar_stack_traces() {
    let mut dedup = Deduplicator::new();
    let pane_id = Uuid::new_v4();

    let trace1 = make_entry(
        pane_id,
        1,
        "ERROR: NullPointerException at Controller.java:45",
    );
    let trace2 = make_entry(
        pane_id,
        2,
        "ERROR: NullPointerException at Controller.java:48",
    );

    let sig = generate_signature(&trace1.raw_content);
    dedup.add_signature(&trace1, sig);

    // The second trace (only line number differs) should be flagged as duplicate
    assert!(
        dedup.find_duplicate(&trace2).is_some(),
        "near-duplicate stack trace (only line number differs) should be flagged"
    );
}

// ---------------------------------------------------------------------------
// 4. Severity ordering
// ---------------------------------------------------------------------------

/// Ingest a mix of severity levels and verify that `get_entries_by_severity`
/// returns exactly the entries matching each queried severity.
#[tokio::test]
async fn test_severity_ordering_get_by_severity() {
    let manager = BufferManager::new_in_memory(/*capacity=*/ 500, /*retention_minutes=*/ 60);
    let pane_id = Uuid::new_v4();
    manager.create_buffer(pane_id).await;

    let entries = vec![
        make_entry_with_severity(pane_id, 1, "trace msg", Severity::Trace),
        make_entry_with_severity(pane_id, 2, "debug msg", Severity::Debug),
        make_entry_with_severity(pane_id, 3, "info msg", Severity::Info),
        make_entry_with_severity(pane_id, 4, "warn msg", Severity::Warn),
        make_entry_with_severity(pane_id, 5, "error msg", Severity::Error),
        make_entry_with_severity(pane_id, 6, "fatal msg", Severity::Fatal),
    ];

    for e in entries {
        manager.add_entry(e).await.expect("add_entry should succeed");
    }

    // Each severity query must return exactly one entry
    for sev in [
        Severity::Trace,
        Severity::Debug,
        Severity::Info,
        Severity::Warn,
        Severity::Error,
        Severity::Fatal,
    ] {
        let results = manager.get_entries_by_severity(sev).await;
        assert_eq!(
            results.len(),
            1,
            "expected exactly 1 entry for {:?}, got {}",
            sev,
            results.len()
        );
        assert_eq!(
            results[0].severity, sev,
            "returned entry severity mismatch for {:?}",
            sev
        );
    }
}

/// Verify the ordering contract: higher-severity entries compare as greater.
#[tokio::test]
async fn test_severity_ordering_contract() {
    // Severity derives PartialOrd/Ord; validate the ordering used by the buffer
    assert!(
        Severity::Fatal > Severity::Error,
        "FATAL must be greater than ERROR"
    );
    assert!(
        Severity::Error > Severity::Warn,
        "ERROR must be greater than WARN"
    );
    assert!(
        Severity::Warn > Severity::Info,
        "WARN must be greater than INFO"
    );
    assert!(
        Severity::Info > Severity::Debug,
        "INFO must be greater than DEBUG"
    );
    assert!(
        Severity::Debug > Severity::Trace,
        "DEBUG must be greater than TRACE"
    );
}

// ---------------------------------------------------------------------------
// 5. Pipeline parse → buffer ingest flow
// ---------------------------------------------------------------------------

/// Simulate the full parse → ingest chain: run raw log lines through the
/// `LogParser` to set severity, then push them into a `BufferManager` and
/// verify that severity-filtered retrieval works end-to-end.
#[tokio::test]
async fn test_pipeline_parse_then_ingest() {
    let parser = LogParser::new();
    let manager = BufferManager::new_in_memory(500, 60);
    let pane_id = Uuid::new_v4();
    manager.create_buffer(pane_id).await;

    let raw_lines = vec![
        "2024-01-15T10:30:00Z [api] ERROR: request timeout",
        "2024-01-15T10:30:01Z [api] INFO: request received",
        "2024-01-15T10:30:02Z [api] FATAL: out of memory",
        "2024-01-15T10:30:03Z [api] WARN: slow query detected",
    ];

    for (seq, line) in raw_lines.iter().enumerate() {
        let mut entry = make_entry(pane_id, seq as u64 + 1, line);
        parser.parse(&mut entry);
        manager.add_entry(entry).await.expect("add_entry should succeed");
    }

    // Total entries in ring buffer
    let all = manager.get_entries(pane_id).await;
    assert_eq!(all.len(), 4, "all 4 lines should be in the ring buffer");

    // Only one ERROR entry
    let errors = manager.get_entries_by_severity(Severity::Error).await;
    assert_eq!(errors.len(), 1);
    assert!(errors[0].raw_content.contains("request timeout"));

    // Only one FATAL entry
    let fatals = manager.get_entries_by_severity(Severity::Fatal).await;
    assert_eq!(fatals.len(), 1);
    assert!(fatals[0].raw_content.contains("out of memory"));
}
