use crate::error::{LogPilotError, Result};
use once_cell::sync::Lazy;
use std::process::{Command as StdCommand, Stdio};
use tokio::process::Command;

static VALID_TARGET_RE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"^[a-zA-Z0-9_\-\.:%]+$").unwrap());

/// tmux command builder and executor
pub struct TmuxCommand;

/// Validates a tmux session/pane identifier
/// Only allows alphanumeric, hyphens, underscores, dots, and colons
fn validate_target(target: &str) -> Result<()> {
    // tmux targets typically look like: "session", "session:window", "session:window.pane"
    // Allowed characters: alphanumeric, hyphen, underscore, dot, colon

    if !VALID_TARGET_RE.is_match(target) {
        return Err(LogPilotError::tmux(format!(
            "Invalid tmux target '{}': contains potentially dangerous characters",
            target
        )));
    }

    // Check for shell metacharacters that might slip through
    let dangerous_chars = [
        ';', '|', '&', '$', '`', '(', ')', '<', '>', '{', '}', '[', ']', '*', '?', '#', '!', ' ',
        '\t', '\n', '\r', '"', '\'', '\\',
    ];
    for ch in dangerous_chars {
        if target.contains(ch) {
            return Err(LogPilotError::tmux(format!(
                "Invalid tmux target '{}': contains shell metacharacter '{}'",
                target, ch
            )));
        }
    }

    Ok(())
}

/// Validates and sanitizes a file path to prevent path traversal
fn validate_path(path: &str) -> Result<()> {
    // Reject paths with traversal sequences
    if path.contains("..") {
        return Err(LogPilotError::tmux(format!(
            "Invalid path '{}': path traversal not allowed",
            path
        )));
    }

    // Reject paths starting with shell metacharacters
    let dangerous_start_chars = [';', '|', '&', '$', '`', '(', ')', '<', '>'];
    if let Some(first) = path.chars().next() {
        if dangerous_start_chars.contains(&first) {
            return Err(LogPilotError::tmux(format!(
                "Invalid path '{}': starts with dangerous character '{}'",
                path, first
            )));
        }
    }

    // Check for embedded shell commands
    let shell_indicators = ["$(", "`", "|", ";", "&&", "||"];
    for indicator in &shell_indicators {
        if path.contains(indicator) {
            return Err(LogPilotError::tmux(format!(
                "Invalid path '{}': contains shell command indicator '{}'",
                path, indicator
            )));
        }
    }

    Ok(())
}

impl TmuxCommand {
    /// Execute tmux pipe-pane to start streaming output
    ///
    /// SECURITY: Both target and output_path are validated to prevent command injection.
    /// The target must be a valid tmux identifier (alphanumeric + limited punctuation).
    /// The output_path is validated to prevent path traversal and shell injection.
    pub async fn pipe_pane(target: &str, output_path: &str) -> Result<tokio::process::Child> {
        // Validate inputs to prevent command injection
        validate_target(target)?;
        validate_path(output_path)?;

        // Use exec to avoid shell interpretation and properly quote the path
        let cmd = format!("exec cat >> '{}'", output_path.replace('\'', "'\"'\"'"));

        let child = Command::new("tmux")
            .args(["pipe-pane", "-t", target, &cmd])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(LogPilotError::Io)?;

        Ok(child)
    }

    /// Stop piping from a pane
    pub async fn stop_pipe(target: &str) -> Result<()> {
        validate_target(target)?;

        let output = Command::new("tmux")
            .args(["pipe-pane", "-t", target])
            .output()
            .await
            .map_err(LogPilotError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(LogPilotError::tmux(format!(
                "Failed to stop pipe-pane for {}: {}",
                target, stderr
            )));
        }

        Ok(())
    }

    /// List all tmux sessions
    pub async fn list_sessions() -> Result<Vec<String>> {
        let output = Command::new("tmux")
            .args(["list-sessions", "-F", "#S"])
            .output()
            .await
            .map_err(LogPilotError::Io)?;

        if !output.status.success() {
            // No sessions is not an error, just empty list
            if String::from_utf8_lossy(&output.stderr).contains("no server running") {
                return Ok(Vec::new());
            }
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(LogPilotError::tmux(format!(
                "list-sessions failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let sessions: Vec<String> = stdout
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            // Filter out any session names with suspicious characters
            .filter(|s| validate_target(s).is_ok())
            .collect();

        Ok(sessions)
    }

    /// List all panes for a session (across all windows)
    pub async fn list_panes(session: &str) -> Result<Vec<String>> {
        validate_target(session)?;

        // Get all windows in the session first
        let windows_output = Command::new("tmux")
            .args(["list-windows", "-t", session, "-F", "#I"])
            .output()
            .await
            .map_err(LogPilotError::Io)?;

        if !windows_output.status.success() {
            let stderr = String::from_utf8_lossy(&windows_output.stderr);
            return Err(LogPilotError::tmux(format!(
                "list-windows failed for {}: {}",
                session, stderr
            )));
        }

        let windows_stdout = String::from_utf8_lossy(&windows_output.stdout);
        let windows: Vec<String> = windows_stdout
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // Get panes from each window
        let mut all_panes = Vec::new();
        for window in windows {
            let target = format!("{}:{}", session, window);
            let output = Command::new("tmux")
                .args(["list-panes", "-t", &target, "-F", "#D"])
                .output()
                .await
                .map_err(LogPilotError::Io)?;

            if !output.status.success() {
                continue; // Skip windows that fail
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            let panes: Vec<String> = stdout
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            all_panes.extend(panes);
        }

        Ok(all_panes)
    }

    /// Check if a session exists
    pub async fn session_exists(name: &str) -> Result<bool> {
        // Validate the name first
        validate_target(name)?;
        let sessions = Self::list_sessions().await?;
        Ok(sessions.contains(&name.to_string()))
    }

    /// Get the active pane for a session
    pub async fn get_active_pane(session: &str) -> Result<String> {
        validate_target(session)?;

        let output = Command::new("tmux")
            .args(["list-panes", "-t", session, "-F", "#D #F"])
            .output()
            .await
            .map_err(LogPilotError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(LogPilotError::tmux(format!(
                "Failed to get active pane for {}: {}",
                session, stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Find pane marked with * (active)
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1].contains('*') {
                return Ok(parts[0].to_string());
            }
        }

        // Fallback to first pane
        stdout
            .lines()
            .next()
            .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| LogPilotError::tmux(format!("No panes found in session {}", session)))
    }

    /// Check if tmux is installed
    pub fn is_installed() -> bool {
        StdCommand::new("tmux")
            .arg("-V")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tmux_installed_check() {
        // This test just verifies the function doesn't panic
        let _installed = TmuxCommand::is_installed();
    }

    #[test]
    fn test_validate_target_valid() {
        // Valid targets should pass
        assert!(validate_target("my-session").is_ok());
        assert!(validate_target("session_1").is_ok());
        assert!(validate_target("api:1.0").is_ok());
        assert!(validate_target("my-app-v2").is_ok());
        // Tmux pane IDs start with %
        assert!(validate_target("%17").is_ok());
        assert!(validate_target("%0").is_ok());
    }

    #[test]
    fn test_validate_target_invalid() {
        // Invalid targets with shell metacharacters should fail
        assert!(validate_target("session;rm -rf /").is_err());
        assert!(validate_target("session|cat /etc/passwd").is_err());
        assert!(validate_target("session$(whoami)").is_err());
        assert!(validate_target("session`id`").is_err());
        assert!(validate_target("session & disown").is_err());
    }

    #[test]
    fn test_validate_path_valid() {
        assert!(validate_path("/tmp/test.log").is_ok());
        assert!(validate_path("./logs/output.txt").is_ok());
        assert!(validate_path("/home/user/.config/logpilot/data.db").is_ok());
    }

    #[test]
    fn test_validate_path_traversal() {
        // Path traversal attempts should fail
        assert!(validate_path("../etc/passwd").is_err());
        assert!(validate_path("/tmp/../etc/shadow").is_err());
        assert!(validate_path("..\\windows\\system32").is_err());
    }

    #[test]
    fn test_validate_path_shell_injection() {
        // Shell injection attempts should fail
        assert!(validate_path("; rm -rf /").is_err());
        assert!(validate_path("| cat /etc/passwd").is_err());
        assert!(validate_path("$(whoami)").is_err());
        assert!(validate_path("`id`").is_err());
    }
}
