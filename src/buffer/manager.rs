//! Buffer manager for per-pane log storage
//!
//! Combines in-memory ring buffer with SQLite persistence

use crate::buffer::persistence::PersistenceStore;
use crate::buffer::ring::RingBuffer;
use crate::error::Result;
use crate::models::{LogEntry, Severity};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Manages buffers for multiple panes
pub struct BufferManager {
    /// Per-pane ring buffers
    buffers: RwLock<HashMap<Uuid, RingBuffer>>,
    /// Optional persistence store
    persistence: Option<PersistenceStore>,
    /// Default buffer capacity
    capacity: usize,
    /// Default retention in minutes
    retention_minutes: u32,
    /// Minimum severity for persistence
    persist_severity: Severity,
}

impl BufferManager {
    /// Create a new buffer manager without persistence
    pub fn new_in_memory(capacity: usize, retention_minutes: u32) -> Self {
        Self {
            buffers: RwLock::new(HashMap::new()),
            persistence: None,
            capacity,
            retention_minutes,
            persist_severity: Severity::Error,
        }
    }

    /// Create a new buffer manager with persistence
    pub async fn with_persistence(
        db_path: &str,
        capacity: usize,
        retention_minutes: u32,
        persist_severity: Severity,
    ) -> Result<Self> {
        let persistence = PersistenceStore::new(db_path).await?;

        Ok(Self {
            buffers: RwLock::new(HashMap::new()),
            persistence: Some(persistence),
            capacity,
            retention_minutes,
            persist_severity,
        })
    }

    /// Create buffer for a pane
    pub async fn create_buffer(&self, pane_id: Uuid) {
        let mut buffers = self.buffers.write().await;
        buffers.insert(
            pane_id,
            RingBuffer::new(self.capacity, self.retention_minutes),
        );
    }

    /// Remove buffer for a pane
    pub async fn remove_buffer(&self, pane_id: Uuid) {
        let mut buffers = self.buffers.write().await;
        buffers.remove(&pane_id);
    }

    /// Add entry to pane's buffer
    pub async fn add_entry(&self, entry: LogEntry) -> Result<()> {
        let pane_id = entry.pane_id;

        // Ensure buffer exists
        {
            let buffers = self.buffers.read().await;
            if !buffers.contains_key(&pane_id) {
                drop(buffers);
                self.create_buffer(pane_id).await;
            }
        }

        // Add to ring buffer
        {
            let mut buffers = self.buffers.write().await;
            if let Some(buffer) = buffers.get_mut(&pane_id) {
                buffer.push(entry.clone());
            }
        }

        // Persist if severity threshold met
        if let Some(ref persistence) = self.persistence {
            if entry.severity >= self.persist_severity {
                persistence
                    .store_entry(&entry, self.persist_severity)
                    .await?;
            }
        }

        Ok(())
    }

    /// Get entries for a pane
    pub async fn get_entries(&self, pane_id: Uuid) -> Vec<LogEntry> {
        let buffers = self.buffers.read().await;
        if let Some(buffer) = buffers.get(&pane_id) {
            buffer.entries().iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Get entries since timestamp for a pane
    pub async fn get_entries_since(&self, pane_id: Uuid, since: DateTime<Utc>) -> Vec<LogEntry> {
        let buffers = self.buffers.read().await;
        if let Some(buffer) = buffers.get(&pane_id) {
            buffer.entries_since(since).into_iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Get all entries across all panes
    pub async fn get_all_entries(&self) -> Vec<LogEntry> {
        let buffers = self.buffers.read().await;
        let mut all = Vec::new();
        for buffer in buffers.values() {
            all.extend(buffer.entries().iter().cloned());
        }
        all
    }

    /// Get entries by severity across all panes
    pub async fn get_entries_by_severity(&self, severity: Severity) -> Vec<LogEntry> {
        let buffers = self.buffers.read().await;
        let mut entries = Vec::new();
        for buffer in buffers.values() {
            entries.extend(buffer.entries_by_severity(severity).into_iter().cloned());
        }
        entries
    }

    /// Cleanup old entries
    pub async fn cleanup(&self) {
        let mut buffers = self.buffers.write().await;
        for buffer in buffers.values_mut() {
            buffer.cleanup();
        }
    }

    /// Query persisted entries by time range
    pub async fn query_persisted(
        &self,
        since: DateTime<Utc>,
        until: DateTime<Utc>,
        severity: Option<Severity>,
    ) -> Result<Vec<LogEntry>> {
        if let Some(ref persistence) = self.persistence {
            persistence.query_entries(since, until, severity).await
        } else {
            Ok(Vec::new())
        }
    }

    /// Get buffer statistics
    pub async fn stats(&self) -> BufferStats {
        let buffers = self.buffers.read().await;
        let mut total_entries = 0;
        let mut total_capacity = 0;

        for buffer in buffers.values() {
            total_entries += buffer.len();
            total_capacity += buffer.capacity();
        }

        BufferStats {
            pane_count: buffers.len(),
            total_entries,
            total_capacity,
            persistence_enabled: self.persistence.is_some(),
        }
    }

    /// Clear all buffers
    pub async fn clear_all(&self) {
        let mut buffers = self.buffers.write().await;
        for buffer in buffers.values_mut() {
            buffer.clear();
        }
        buffers.clear();
    }
}

/// Buffer statistics
#[derive(Debug, Clone)]
pub struct BufferStats {
    pub pane_count: usize,
    pub total_entries: usize,
    pub total_capacity: usize,
    pub persistence_enabled: bool,
}

impl BufferStats {
    pub fn utilization_percent(&self) -> f64 {
        if self.total_capacity == 0 {
            0.0
        } else {
            (self.total_entries as f64 / self.total_capacity as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_entry(pane_id: Uuid, content: &str, severity: Severity) -> LogEntry {
        LogEntry {
            id: Uuid::new_v4(),
            pane_id,
            sequence: 1,
            timestamp: Utc::now(),
            severity,
            service: None,
            raw_content: content.to_string(),
            parsed_fields: HashMap::new(),
            received_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_buffer_manager_add_and_get() {
        let manager = BufferManager::new_in_memory(100, 30);
        let pane_id = Uuid::new_v4();

        manager.create_buffer(pane_id).await;

        let entry = create_test_entry(pane_id, "test", Severity::Info);
        manager.add_entry(entry.clone()).await.unwrap();

        let entries = manager.get_entries(pane_id).await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_content, "test");
    }

    #[tokio::test]
    async fn test_buffer_manager_get_by_severity() {
        let manager = BufferManager::new_in_memory(100, 30);
        let pane_id = Uuid::new_v4();

        manager.create_buffer(pane_id).await;

        manager
            .add_entry(create_test_entry(pane_id, "info", Severity::Info))
            .await
            .unwrap();
        manager
            .add_entry(create_test_entry(pane_id, "error", Severity::Error))
            .await
            .unwrap();
        manager
            .add_entry(create_test_entry(pane_id, "fatal", Severity::Fatal))
            .await
            .unwrap();

        let errors = manager.get_entries_by_severity(Severity::Error).await;
        assert_eq!(errors.len(), 1);
        assert!(errors[0].raw_content.contains("error"));

        let fatals = manager.get_entries_by_severity(Severity::Fatal).await;
        assert_eq!(fatals.len(), 1);
    }

    #[tokio::test]
    async fn test_buffer_manager_stats() {
        let manager = BufferManager::new_in_memory(100, 30);
        let pane_id = Uuid::new_v4();

        manager.create_buffer(pane_id).await;

        for i in 0..50 {
            let mut entry = create_test_entry(pane_id, &format!("entry {}", i), Severity::Info);
            entry.sequence = i;
            manager.add_entry(entry).await.unwrap();
        }

        let stats = manager.stats().await;
        assert_eq!(stats.pane_count, 1);
        assert_eq!(stats.total_entries, 50);
        assert_eq!(stats.total_capacity, 100);
        assert!(!stats.persistence_enabled);
        assert_eq!(stats.utilization_percent(), 50.0);
    }

    #[tokio::test]
    async fn test_buffer_manager_cleanup() {
        let manager = BufferManager::new_in_memory(100, 1); // 1 minute retention
        let pane_id = Uuid::new_v4();

        manager.create_buffer(pane_id).await;

        // Add old entry
        let mut old_entry = create_test_entry(pane_id, "old", Severity::Info);
        old_entry.timestamp = Utc::now() - chrono::Duration::minutes(5);
        manager.add_entry(old_entry).await.unwrap();

        // Add new entry
        manager
            .add_entry(create_test_entry(pane_id, "new", Severity::Info))
            .await
            .unwrap();

        // Cleanup
        manager.cleanup().await;

        // Only new entry should remain
        let entries = manager.get_entries(pane_id).await;
        assert_eq!(entries.len(), 1);
        assert!(entries[0].raw_content.contains("new"));
    }
}
