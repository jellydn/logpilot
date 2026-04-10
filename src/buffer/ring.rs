//! Ring buffer implementation for in-memory log storage
//!
//! Provides O(1) insertion and eviction with circular buffer semantics

use crate::models::LogEntry;
use std::collections::VecDeque;

/// Fixed-size ring buffer with time-based eviction
pub struct RingBuffer {
    capacity: usize,
    buffer: VecDeque<LogEntry>,
    /// Retention duration in minutes
    retention_minutes: u32,
}

impl RingBuffer {
    /// Create a new ring buffer with specified capacity and retention
    pub fn new(capacity: usize, retention_minutes: u32) -> Self {
        Self {
            capacity,
            buffer: VecDeque::with_capacity(capacity),
            retention_minutes,
        }
    }

    /// Add an entry to the buffer
    pub fn push(&mut self, entry: LogEntry) {
        // Remove oldest if at capacity
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(entry);
    }

    /// Get all entries in the buffer
    pub fn entries(&self) -> &VecDeque<LogEntry> {
        &self.buffer
    }

    /// Get entries since a specific timestamp
    pub fn entries_since(&self, since: chrono::DateTime<chrono::Utc>) -> Vec<&LogEntry> {
        self.buffer
            .iter()
            .filter(|e| e.timestamp >= since)
            .collect()
    }

    /// Get entries for a specific pane
    pub fn entries_for_pane(&self, pane_id: uuid::Uuid) -> Vec<&LogEntry> {
        self.buffer
            .iter()
            .filter(|e| e.pane_id == pane_id)
            .collect()
    }

    /// Clean up old entries based on retention policy
    pub fn cleanup(&mut self) {
        let cutoff = chrono::Utc::now() - chrono::Duration::minutes(self.retention_minutes as i64);
        while let Some(entry) = self.buffer.front() {
            if entry.timestamp < cutoff {
                self.buffer.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get buffer size
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Get entries by severity
    pub fn entries_by_severity(&self, severity: crate::models::Severity) -> Vec<&LogEntry> {
        self.buffer
            .iter()
            .filter(|e| e.severity == severity)
            .collect()
    }

    /// Get the newest entry
    pub fn newest(&self) -> Option<&LogEntry> {
        self.buffer.back()
    }

    /// Get the oldest entry
    pub fn oldest(&self) -> Option<&LogEntry> {
        self.buffer.front()
    }
}

impl Default for RingBuffer {
    fn default() -> Self {
        Self::new(10000, 30) // Default: 10k entries, 30 min retention
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Severity;
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn create_test_entry(content: &str, severity: Severity) -> LogEntry {
        LogEntry {
            id: Uuid::new_v4(),
            pane_id: Uuid::new_v4(),
            sequence: 1,
            timestamp: Utc::now(),
            severity,
            service: None,
            raw_content: content.to_string(),
            parsed_fields: HashMap::new(),
            received_at: Utc::now(),
        }
    }

    #[test]
    fn test_ring_buffer_push_and_get() {
        let mut buffer = RingBuffer::new(10, 30);
        let entry = create_test_entry("test", Severity::Info);

        buffer.push(entry.clone());

        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.newest().unwrap().raw_content, "test");
    }

    #[test]
    fn test_ring_buffer_capacity_eviction() {
        let mut buffer = RingBuffer::new(5, 30);

        for i in 0..10 {
            buffer.push(create_test_entry(&format!("entry {}", i), Severity::Info));
        }

        assert_eq!(buffer.len(), 5);
        // Oldest should be entry 5
        assert!(buffer.oldest().unwrap().raw_content.contains("entry 5"));
    }

    #[test]
    fn test_ring_buffer_entries_by_severity() {
        let mut buffer = RingBuffer::new(10, 30);

        buffer.push(create_test_entry("info msg", Severity::Info));
        buffer.push(create_test_entry("error msg", Severity::Error));
        buffer.push(create_test_entry("fatal msg", Severity::Fatal));

        let errors = buffer.entries_by_severity(Severity::Error);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].raw_content.contains("error"));

        let fatals = buffer.entries_by_severity(Severity::Fatal);
        assert_eq!(fatals.len(), 1);
    }

    #[test]
    fn test_ring_buffer_entries_since() {
        let mut buffer = RingBuffer::new(10, 30);
        let now = Utc::now();

        // Add old entry
        let mut old_entry = create_test_entry("old", Severity::Info);
        old_entry.timestamp = now - chrono::Duration::minutes(10);
        buffer.push(old_entry);

        // Add new entry
        let new_entry = create_test_entry("new", Severity::Info);
        buffer.push(new_entry);

        // Get entries since 5 minutes ago
        let recent = buffer.entries_since(now - chrono::Duration::minutes(5));
        assert_eq!(recent.len(), 1);
        assert!(recent[0].raw_content.contains("new"));
    }

    #[test]
    fn test_ring_buffer_cleanup() {
        let mut buffer = RingBuffer::new(10, 1); // 1 minute retention
        let now = Utc::now();

        // Add old entry
        let mut old_entry = create_test_entry("old", Severity::Info);
        old_entry.timestamp = now - chrono::Duration::minutes(5);
        buffer.push(old_entry);

        // Add new entry
        let new_entry = create_test_entry("new", Severity::Info);
        buffer.push(new_entry);

        assert_eq!(buffer.len(), 2);

        // Cleanup
        buffer.cleanup();

        // Old entry should be removed
        assert_eq!(buffer.len(), 1);
        assert!(buffer.entries()[0].raw_content.contains("new"));
    }
}
