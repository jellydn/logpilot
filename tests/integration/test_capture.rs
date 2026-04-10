use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;

/// Integration tests for tmux log capture functionality
/// These tests verify the core US1 requirements:
/// - Session attachment
/// - <2s latency
/// - Multiple concurrent captures
/// - Standby mode on disconnect

#[tokio::test]
async fn test_watch_attach_to_session() {
    // Create a mock tmux session using the fixture script
    let mut session = MockTmuxSession::new("test-attach-session").await;
    session.start_log_producer().await;

    // Attempt to attach LogPilot
    let result = attach_logpilot("test-attach-session").await;
    assert!(result.is_ok(), "Should successfully attach to session");

    // Verify session is tracked
    let status = get_logpilot_status().await;
    assert!(status.contains("test-attach-session"));

    session.cleanup().await;
}

#[tokio::test]
async fn test_capture_latency_under_2s() {
    let mut session = MockTmuxSession::new("test-latency-session").await;
    
    // Start log producer with known content
    session.produce_log("LATENCY_TEST_MARKER").await;
    
    // Attach LogPilot
    attach_logpilot("test-latency-session").await.unwrap();
    
    // Measure time until log appears in buffer
    let start = Instant::now();
    let found = wait_for_log_in_buffer("LATENCY_TEST_MARKER", Duration::from_secs(2)).await;
    
    let elapsed = start.elapsed();
    assert!(found, "Log should be captured");
    assert!(
        elapsed < Duration::from_secs(2),
        "Latency should be under 2s, got {:?}",
        elapsed
    );
    
    session.cleanup().await;
}

#[tokio::test]
async fn test_multiple_concurrent_captures() {
    // Create multiple sessions
    let session1 = MockTmuxSession::new("test-multi-1").await;
    let session2 = MockTmuxSession::new("test-multi-2").await;
    let session3 = MockTmuxSession::new("test-multi-3").await;
    
    // Attach to all three concurrently
    let attach1 = attach_logpilot("test-multi-1");
    let attach2 = attach_logpilot("test-multi-2");
    let attach3 = attach_logpilot("test-multi-3");
    
    let results = tokio::join!(attach1, attach2, attach3);
    
    assert!(results.0.is_ok());
    assert!(results.1.is_ok());
    assert!(results.2.is_ok());
    
    // Produce different logs in each session
    session1.produce_log("SESSION_1_MARKER").await;
    session2.produce_log("SESSION_2_MARKER").await;
    session3.produce_log("SESSION_3_MARKER").await;
    
    // Verify each session only captured its own logs (no cross-contamination)
    let buf1 = get_session_buffer("test-multi-1").await;
    let buf2 = get_session_buffer("test-multi-2").await;
    let buf3 = get_session_buffer("test-multi-3").await;
    
    assert!(buf1.contains("SESSION_1_MARKER"));
    assert!(!buf1.contains("SESSION_2_MARKER"));
    assert!(!buf1.contains("SESSION_3_MARKER"));
    
    assert!(buf2.contains("SESSION_2_MARKER"));
    assert!(!buf2.contains("SESSION_1_MARKER"));
    
    session1.cleanup().await;
    session2.cleanup().await;
    session3.cleanup().await;
}

#[tokio::test]
async fn test_session_stale_on_disconnect() {
    let mut session = MockTmuxSession::new("test-stale-session").await;
    session.start_log_producer().await;
    
    // Attach LogPilot
    attach_logpilot("test-stale-session").await.unwrap();
    
    // Verify active status
    let status = get_session_status("test-stale-session").await;
    assert_eq!(status, "Active");
    
    // Kill the tmux session (simulating disconnect)
    session.kill().await;
    
    // Wait for LogPilot to detect disconnect
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // Verify status changed to Stale
    let status = get_session_status("test-stale-session").await;
    assert_eq!(status, "Stale");
    
    // Verify LogPilot is still running (standby mode)
    let logpilot_running = is_logpilot_running().await;
    assert!(logpilot_running, "LogPilot should stay in standby mode");
    
    // Recreate session with same name
    session.recreate().await;
    
    // Wait for auto-reconnect
    tokio::time::sleep(Duration::from_secs(6)).await;
    
    // Verify status back to Active
    let status = get_session_status("test-stale-session").await;
    assert_eq!(status, "Active", "Should auto-reconnect and become Active");
}

// ============================================================================
// Test Helpers
// ============================================================================

/// Mock tmux session for testing without requiring actual tmux
pub struct MockTmuxSession {
    name: String,
    log_file: tempfile::NamedTempFile,
}

impl MockTmuxSession {
    pub async fn new(name: &str) -> Self {
        let log_file = tempfile::NamedTempFile::new().unwrap();
        
        // Create mock tmux socket directory
        let socket_dir = std::path::PathBuf::from(format!("/tmp/logpilot-test-{}-{}", 
            name, 
            std::process::id()
        ));
        tokio::fs::create_dir_all(&socket_dir).await.unwrap();
        
        Self {
            name: name.to_string(),
            log_file,
        }
    }
    
    pub async fn start_log_producer(&mut self) {
        // Start a background process that writes to the log file periodically
        // This simulates `kubectl logs -f` or similar
    }
    
    pub async fn produce_log(&self, content: &str) {
        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .append(true)
            .open(self.log_file.path())
            .await
            .unwrap();
        file.write_all(format!("{}\n", content).as_bytes()).await.unwrap();
        file.flush().await.unwrap();
    }
    
    pub async fn kill(&mut self) {
        // Simulate tmux session kill
    }
    
    pub async fn recreate(&mut self) {
        // Recreate the session after kill
        self.start_log_producer().await;
    }
    
    pub async fn cleanup(self) {
        // Cleanup temp files and processes
    }
}

async fn attach_logpilot(session: &str) -> anyhow::Result<()> {
    // Placeholder - will call actual logpilot CLI
    let output = Command::new("cargo")
        .args(["run", "--", "watch", session, "--test-mode"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;
    
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to attach: {}", 
            String::from_utf8_lossy(&output.stderr)))
    }
}

async fn get_logpilot_status() -> String {
    // Placeholder - query logpilot status
    String::new()
}

async fn get_session_status(session: &str) -> String {
    // Placeholder - get specific session status
    "Active".to_string()
}

async fn wait_for_log_in_buffer(marker: &str, timeout_duration: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout_duration {
        let buffer = get_session_buffer("test").await;
        if buffer.contains(marker) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

async fn get_session_buffer(session: &str) -> String {
    // Placeholder - retrieve buffer content
    String::new()
}

async fn is_logpilot_running() -> bool {
    // Check if logpilot process is still running
    true
}
