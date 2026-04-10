//! Shared session data store for live MCP data integration
//!
//! Provides thread-safe access to session data for both the watch command
//! and the MCP server, enabling real-time AI context.

use crate::models::{Alert, Incident, LogEntry, Pattern};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Data for a single monitored session
#[derive(Debug, Clone, Default)]
pub struct SessionData {
    pub entries: Vec<LogEntry>,
    pub patterns: Vec<Pattern>,
    pub incidents: Vec<Incident>,
    pub alerts: Vec<Alert>,
    pub window_start: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl SessionData {
    /// Create new session data with current timestamp
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            entries: Vec::new(),
            patterns: Vec::new(),
            incidents: Vec::new(),
            alerts: Vec::new(),
            window_start: now - chrono::Duration::minutes(30),
            last_updated: now,
        }
    }

    /// Add a log entry and update timestamp
    pub fn add_entry(&mut self, entry: LogEntry) {
        self.entries.push(entry);
        self.last_updated = Utc::now();
        // Limit entries to prevent unbounded growth (keep last 10k)
        if self.entries.len() > 10000 {
            self.entries.drain(0..self.entries.len() - 10000);
        }
    }

    /// Update patterns list
    pub fn set_patterns(&mut self, patterns: Vec<Pattern>) {
        self.patterns = patterns;
        self.last_updated = Utc::now();
    }

    /// Update incidents list
    pub fn set_incidents(&mut self, incidents: Vec<Incident>) {
        self.incidents = incidents;
        self.last_updated = Utc::now();
    }

    /// Update alerts list
    pub fn set_alerts(&mut self, alerts: Vec<Alert>) {
        self.alerts = alerts;
        self.last_updated = Utc::now();
    }

    /// Get entries since a specific time
    pub fn entries_since(&self, since: DateTime<Utc>) -> Vec<LogEntry> {
        self.entries
            .iter()
            .filter(|e| e.timestamp >= since)
            .cloned()
            .collect()
    }

    /// Get the most recent entries (up to n)
    pub fn recent_entries(&self, n: usize) -> Vec<LogEntry> {
        self.entries.iter().rev().take(n).rev().cloned().collect()
    }
}

const SESSION_STALE_MINUTES: i64 = 60; // Sessions not updated in 60 mins are stale

/// Thread-safe shared store for session data
///
/// This store is shared between the watch command (which writes data)
/// and the MCP server (which reads data for AI context).
#[derive(Debug, Clone)]
pub struct SessionDataStore {
    /// Map of session name to session data
    sessions: Arc<DashMap<String, RwLock<SessionData>>>,
}

impl SessionDataStore {
    /// Create a new empty data store
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }

    /// Clean up stale sessions (not updated in SESSION_STALE_MINUTES)
    pub async fn cleanup_stale_sessions(&self) {
        let now = Utc::now();
        let stale_threshold = now - chrono::Duration::minutes(SESSION_STALE_MINUTES);

        // First pass: identify potentially stale sessions
        let potentially_stale: Vec<String> = self
            .sessions
            .iter()
            .filter(|entry| {
                if let Ok(data) = entry.value().try_read() {
                    data.last_updated < stale_threshold
                } else {
                    false // Skip if can't acquire read lock
                }
            })
            .map(|entry| entry.key().clone())
            .collect();

        // Second pass: re-check under write lock before removal to avoid TOCTOU race
        for session_name in potentially_stale {
            if let Some(entry) = self.sessions.get(&session_name) {
                // Re-check staleness with proper locking
                let should_remove = match entry.try_write() {
                    Ok(data) => data.last_updated < stale_threshold,
                    Err(_) => {
                        // If we can't get write lock, session is active - skip
                        false
                    }
                };

                if should_remove {
                    tracing::info!("Removing stale session: {}", session_name);
                    drop(entry); // Release the ref before removing
                    self.sessions.remove(&session_name);
                }
            }
        }
    }

    /// Initialize a new session in the store
    pub async fn create_session(&self, name: &str) {
        let data = SessionData::new();
        self.sessions.insert(name.to_string(), RwLock::new(data));
    }

    /// Remove a session from the store
    pub fn remove_session(&self, name: &str) {
        self.sessions.remove(name);
    }

    /// Add a log entry to a session
    pub async fn add_entry(&self, session_name: &str, entry: LogEntry) {
        if let Some(session) = self.sessions.get(session_name) {
            let mut data = session.write().await;
            data.add_entry(entry);
        }
    }

    /// Update patterns for a session
    pub async fn update_patterns(&self, session_name: &str, patterns: Vec<Pattern>) {
        if let Some(session) = self.sessions.get(session_name) {
            let mut data = session.write().await;
            data.set_patterns(patterns);
        }
    }

    /// Add or update a single pattern
    pub async fn upsert_pattern(&self, session_name: &str, pattern: Pattern) {
        if let Some(session) = self.sessions.get(session_name) {
            let mut data = session.write().await;
            // Replace existing pattern or add new one
            if let Some(existing) = data.patterns.iter_mut().find(|p| p.id == pattern.id) {
                *existing = pattern;
            } else {
                data.patterns.push(pattern);
            }
            data.last_updated = Utc::now();
        }
    }

    /// Update incidents for a session
    pub async fn update_incidents(&self, session_name: &str, incidents: Vec<Incident>) {
        if let Some(session) = self.sessions.get(session_name) {
            let mut data = session.write().await;
            data.set_incidents(incidents);
        }
    }

    /// Add or update a single incident
    pub async fn upsert_incident(&self, session_name: &str, incident: Incident) {
        if let Some(session) = self.sessions.get(session_name) {
            let mut data = session.write().await;
            if let Some(existing) = data.incidents.iter_mut().find(|i| i.id == incident.id) {
                *existing = incident;
            } else {
                data.incidents.push(incident);
            }
            data.last_updated = Utc::now();
        }
    }

    /// Update alerts for a session
    pub async fn update_alerts(&self, session_name: &str, alerts: Vec<Alert>) {
        if let Some(session) = self.sessions.get(session_name) {
            let mut data = session.write().await;
            data.set_alerts(alerts);
        }
    }

    /// Add or update a single alert
    pub async fn upsert_alert(&self, session_name: &str, alert: Alert) {
        if let Some(session) = self.sessions.get(session_name) {
            let mut data = session.write().await;
            if let Some(existing) = data.alerts.iter_mut().find(|a| a.id == alert.id) {
                *existing = alert;
            } else {
                data.alerts.push(alert);
            }
            data.last_updated = Utc::now();
        }
    }

    /// Get session data (read-only)
    pub async fn get_session(&self, name: &str) -> Option<SessionData> {
        let session = self.sessions.get(name)?;
        let data = session.read().await.clone();
        drop(session);
        Some(data)
    }

    /// Get all session names
    pub fn list_sessions(&self) -> Vec<String> {
        self.sessions
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Check if a session exists
    pub fn has_session(&self, name: &str) -> bool {
        self.sessions.contains_key(name)
    }

    /// Get the number of active sessions
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get session statistics
    pub async fn get_stats(&self, name: &str) -> Option<SessionStats> {
        if let Some(session) = self.sessions.get(name) {
            let data = session.read().await;
            Some(SessionStats {
                entry_count: data.entries.len(),
                pattern_count: data.patterns.len(),
                incident_count: data.incidents.len(),
                alert_count: data.alerts.len(),
                last_updated: data.last_updated,
                window_start: data.window_start,
            })
        } else {
            None
        }
    }

    /// Clean up old entries across all sessions
    pub async fn cleanup_old_entries(&self, max_age: chrono::Duration) {
        let cutoff = Utc::now() - max_age;
        for session in self.sessions.iter() {
            let mut data = session.write().await;
            data.entries.retain(|e| e.timestamp >= cutoff);
        }
    }
}

impl Default for SessionDataStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for a session
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub entry_count: usize,
    pub pattern_count: usize,
    pub incident_count: usize,
    pub alert_count: usize,
    pub last_updated: DateTime<Utc>,
    pub window_start: DateTime<Utc>,
}

/// Global singleton for the session data store
///
/// This provides a global access point for the shared store,
/// which is needed because the watch command and MCP server
/// run in different contexts.
use once_cell::sync::OnceCell;

static GLOBAL_DATA_STORE: OnceCell<SessionDataStore> = OnceCell::new();

/// Initialize the global data store (call once at startup)
pub fn init_global_store() -> SessionDataStore {
    let store = SessionDataStore::new();
    let _ = GLOBAL_DATA_STORE.set(store.clone());
    store
}

/// Get the global data store (returns None if not initialized)
pub fn global_store() -> Option<SessionDataStore> {
    GLOBAL_DATA_STORE.get().cloned()
}

/// Get the global data store, initializing if needed
pub fn get_or_init_global_store() -> SessionDataStore {
    GLOBAL_DATA_STORE.get_or_init(SessionDataStore::new).clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Alert, AlertType, Severity};
    use uuid::Uuid;

    fn create_test_entry(content: &str) -> LogEntry {
        LogEntry::new_with_severity(
            Uuid::new_v4(),
            1,
            Utc::now(),
            content.to_string(),
            Severity::Info,
        )
    }

    #[tokio::test]
    async fn test_create_and_get_session() {
        let store = SessionDataStore::new();
        store.create_session("test-session").await;

        let data = store.get_session("test-session").await;
        assert!(data.is_some());
    }

    #[tokio::test]
    async fn test_add_entry() {
        let store = SessionDataStore::new();
        store.create_session("test-session").await;

        let entry = create_test_entry("test log line");
        store.add_entry("test-session", entry.clone()).await;

        let data = store.get_session("test-session").await.unwrap();
        assert_eq!(data.entries.len(), 1);
        assert_eq!(data.entries[0].raw_content, "test log line");
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let store = SessionDataStore::new();
        store.create_session("session-1").await;
        store.create_session("session-2").await;

        let sessions = store.list_sessions();
        assert_eq!(sessions.len(), 2);
        assert!(sessions.contains(&"session-1".to_string()));
        assert!(sessions.contains(&"session-2".to_string()));
    }

    #[tokio::test]
    async fn test_remove_session() {
        let store = SessionDataStore::new();
        store.create_session("test-session").await;
        assert!(store.has_session("test-session"));

        store.remove_session("test-session");
        assert!(!store.has_session("test-session"));
    }

    #[tokio::test]
    async fn test_upsert_alert() {
        let store = SessionDataStore::new();
        store.create_session("test-session").await;

        let alert = Alert::new(AlertType::ErrorRate, "test alert".to_string(), 1.0);
        store.upsert_alert("test-session", alert.clone()).await;

        let data = store.get_session("test-session").await.unwrap();
        assert_eq!(data.alerts.len(), 1);
        assert_eq!(data.alerts[0].message, "test alert");

        // Update existing alert
        let mut alert2 = alert.clone();
        alert2.current_value = 5.0;
        store.upsert_alert("test-session", alert2).await;

        let data = store.get_session("test-session").await.unwrap();
        assert_eq!(data.alerts.len(), 1);
        assert_eq!(data.alerts[0].current_value, 5.0);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let store = SessionDataStore::new();
        store.create_session("test-session").await;

        store
            .add_entry("test-session", create_test_entry("entry 1"))
            .await;
        store
            .add_entry("test-session", create_test_entry("entry 2"))
            .await;

        let stats = store.get_stats("test-session").await.unwrap();
        assert_eq!(stats.entry_count, 2);
    }
}
