//! Cluster engine for grouping similar log entries into patterns

use crate::models::LogEntry;
use crate::pipeline::dedup::{generate_signature, Deduplicator};
use std::collections::HashMap;

/// Cluster engine that groups similar entries into patterns
pub struct ClusterEngine {
    /// Deduplicator for signature matching
    deduplicator: Deduplicator,
    /// Map of signature -> representative entry ID
    clusters: HashMap<String, uuid::Uuid>,
}

impl ClusterEngine {
    pub fn new() -> Self {
        Self {
            deduplicator: Deduplicator::new(),
            clusters: HashMap::new(),
        }
    }

    /// Process a log entry and assign it to a cluster
    /// Returns (signature, is_new) where is_new indicates if this created a new cluster
    pub fn cluster(&mut self, entry: &LogEntry) -> (String, bool) {
        // Try to find existing cluster
        if let Some(signature) = self.deduplicator.find_duplicate(entry) {
            // Entry belongs to existing cluster
            return (signature, false);
        }

        // Create new cluster
        let signature = generate_signature(&entry.raw_content);
        self.deduplicator.add_signature(entry, signature.clone());
        self.clusters.insert(signature.clone(), entry.id);

        (signature, true)
    }

    /// Get the representative entry ID for a cluster
    pub fn get_representative(&self, signature: &str) -> Option<uuid::Uuid> {
        self.clusters.get(signature).copied()
    }

    /// Get the number of unique clusters
    pub fn cluster_count(&self) -> usize {
        self.clusters.len()
    }

    /// Check if a signature is a known cluster
    pub fn is_known(&self, signature: &str) -> bool {
        self.clusters.contains_key(signature)
    }
}

impl Default for ClusterEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Multi-entry cluster for grouping related log entries (e.g., stack traces)
pub struct LogCluster {
    pub signature: String,
    pub entries: Vec<uuid::Uuid>,
    pub first_seen: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

impl LogCluster {
    pub fn new(signature: String, first_entry: uuid::Uuid) -> Self {
        let now = chrono::Utc::now();
        Self {
            signature,
            entries: vec![first_entry],
            first_seen: now,
            last_seen: now,
        }
    }

    pub fn add_entry(&mut self, entry_id: uuid::Uuid) {
        self.entries.push(entry_id);
        self.last_seen = chrono::Utc::now();
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

/// Cluster manager that tracks clusters over time
pub struct ClusterManager {
    clusters: HashMap<String, LogCluster>,
    max_clusters: usize,
}

impl ClusterManager {
    pub fn new() -> Self {
        Self {
            clusters: HashMap::new(),
            max_clusters: 10000, // Limit memory usage
        }
    }

    pub fn add_to_cluster(&mut self, signature: String, entry_id: uuid::Uuid, is_new: bool) {
        if is_new {
            // Create new cluster
            if self.clusters.len() >= self.max_clusters {
                // Evict oldest cluster (simple LRU)
                self.evict_oldest();
            }
            let cluster = LogCluster::new(signature.clone(), entry_id);
            self.clusters.insert(signature, cluster);
        } else {
            // Add to existing cluster
            if let Some(cluster) = self.clusters.get_mut(&signature) {
                cluster.add_entry(entry_id);
            }
        }
    }

    fn evict_oldest(&mut self) {
        // Find cluster with earliest last_seen
        if let Some(oldest) = self
            .clusters
            .iter()
            .min_by_key(|(_, c)| c.last_seen)
            .map(|(sig, _)| sig.clone())
        {
            self.clusters.remove(&oldest);
        }
    }

    pub fn get_cluster(&self, signature: &str) -> Option<&LogCluster> {
        self.clusters.get(signature)
    }

    pub fn all_clusters(&self) -> &HashMap<String, LogCluster> {
        &self.clusters
    }

    pub fn active_cluster_count(&self) -> usize {
        self.clusters.len()
    }
}

impl Default for ClusterManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_engine() {
        let mut engine = ClusterEngine::new();

        let entry1 = LogEntry::new(
            uuid::Uuid::new_v4(),
            1,
            chrono::Utc::now(),
            "ERROR: Database connection failed".to_string(),
        );

        let entry2 = LogEntry::new(
            uuid::Uuid::new_v4(),
            2,
            chrono::Utc::now(),
            "ERROR: Database connection failed".to_string(),
        );

        // First entry creates new cluster
        let (sig1, is_new1) = engine.cluster(&entry1);
        assert!(is_new1);

        // Second entry matches existing cluster
        let (sig2, is_new2) = engine.cluster(&entry2);
        assert!(!is_new2);
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_cluster_manager() {
        let mut manager = ClusterManager::new();

        let sig = "abc123".to_string();
        let entry_id = uuid::Uuid::new_v4();

        manager.add_to_cluster(sig.clone(), entry_id, true);

        let cluster = manager.get_cluster(&sig).unwrap();
        assert_eq!(cluster.entry_count(), 1);

        // Add more entries
        for _ in 0..5 {
            manager.add_to_cluster(sig.clone(), uuid::Uuid::new_v4(), false);
        }

        let cluster = manager.get_cluster(&sig).unwrap();
        assert_eq!(cluster.entry_count(), 6);
    }
}
