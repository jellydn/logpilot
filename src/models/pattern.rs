use super::severity::Severity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: Uuid,
    pub signature: String,
    pub regex: Option<String>,
    pub severity: Severity,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub occurrence_count: u64,
    pub window_count: u32,
    pub window_start: DateTime<Utc>,
    pub sample_entry: Option<Uuid>,
}

impl Pattern {
    pub fn new(signature: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            signature: signature.into(),
            regex: None,
            severity: Severity::Unknown,
            first_seen: now,
            last_seen: now,
            occurrence_count: 1,
            window_count: 1,
            window_start: now,
            sample_entry: None,
        }
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_sample_entry(mut self, entry_id: Uuid) -> Self {
        self.sample_entry = Some(entry_id);
        self
    }

    #[cfg(test)]
    pub fn record_occurrence(&mut self) {
        self.occurrence_count += 1;
        self.window_count += 1;
        self.last_seen = Utc::now();
    }

    #[cfg(test)]
    pub fn decay_window(&mut self, window_duration_seconds: i64) {
        let now = Utc::now();
        if now.signed_duration_since(self.window_start).num_seconds() > window_duration_seconds {
            self.window_count = 1; // Start fresh
            self.window_start = now;
        }
    }

    #[cfg(test)]
    pub fn is_recurring(&self, threshold: u32) -> bool {
        self.window_count >= threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_creation() {
        let pattern = Pattern::new("error: connection refused");
        assert_eq!(pattern.occurrence_count, 1);
        assert_eq!(pattern.window_count, 1);
    }

    #[test]
    fn test_pattern_occurrence_tracking() {
        let mut pattern = Pattern::new("test");
        pattern.record_occurrence();
        pattern.record_occurrence();

        assert_eq!(pattern.occurrence_count, 3);
        assert_eq!(pattern.window_count, 3);
    }

    #[test]
    fn test_window_decay() {
        let mut pattern = Pattern::new("test");
        pattern.record_occurrence();
        pattern.record_occurrence();
        assert_eq!(pattern.window_count, 3);

        // Artificially set window start to 70 seconds ago
        pattern.window_start = Utc::now() - chrono::Duration::seconds(70);
        pattern.decay_window(60);

        assert_eq!(pattern.window_count, 1); // Reset to 1 (new window)
    }

    #[test]
    fn test_is_recurring() {
        let mut pattern = Pattern::new("test");
        assert!(!pattern.is_recurring(5));

        for _ in 0..5 {
            pattern.record_occurrence();
        }

        assert!(pattern.is_recurring(5));
    }
}
