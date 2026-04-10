//! Anomaly detection and pattern analysis
#![allow(dead_code)] // Infrastructure not yet wired to CLI

pub mod alerts;
pub mod incidents;
pub mod patterns;

pub use alerts::{AlertEvaluator, ErrorRateCalculator};

use crate::models::LogEntry;
use crate::pipeline::cluster::{ClusterEngine, ClusterManager};
use crate::pipeline::formats::FormatParser;
use crate::pipeline::parser::LogParser;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main analyzer that orchestrates pattern detection and incident creation
pub struct Analyzer {
    parser: LogParser,
    cluster_engine: Arc<RwLock<ClusterEngine>>,
    cluster_manager: Arc<RwLock<ClusterManager>>,
    pattern_tracker: Arc<RwLock<patterns::PatternTracker>>,
    incident_detector: Arc<RwLock<incidents::IncidentDetector>>,
}

impl Analyzer {
    pub fn new() -> Self {
        Self {
            parser: LogParser::new(),
            cluster_engine: Arc::new(RwLock::new(ClusterEngine::new())),
            cluster_manager: Arc::new(RwLock::new(ClusterManager::new())),
            pattern_tracker: Arc::new(RwLock::new(patterns::PatternTracker::new())),
            incident_detector: Arc::new(RwLock::new(incidents::IncidentDetector::new())),
        }
    }

    /// Process a log entry through the full analysis pipeline
    pub async fn process_entry(&self, mut entry: LogEntry) -> AnalysisResult {
        // Step 1: Parse structured formats (JSON, logfmt)
        if !FormatParser::try_parse_json(&mut entry) {
            FormatParser::try_parse_logfmt(&mut entry);
        }

        // Step 2: Apply regex-based parsing
        self.parser.parse(&mut entry);

        // Step 3: Cluster (deduplicate)
        let (signature, is_new_cluster) = {
            let mut engine = self.cluster_engine.write().await;
            engine.cluster(&entry)
        };

        // Add to cluster manager
        {
            let mut manager = self.cluster_manager.write().await;
            manager.add_to_cluster(signature.clone(), entry.id, is_new_cluster);
        }

        // Step 4: Track pattern frequency
        let pattern_state = {
            let tracker = self.pattern_tracker.write().await;
            tracker.track(&signature, &entry).await
        };

        // Step 5: Check for incidents
        let incident = if pattern_state.should_create_incident {
            let detector = self.incident_detector.write().await;
            Some(
                detector
                    .create_incident(&signature, &entry, pattern_state.window_count)
                    .await,
            )
        } else {
            None
        };

        AnalysisResult {
            entry,
            signature,
            is_new_pattern: is_new_cluster,
            window_count: pattern_state.window_count,
            incident,
        }
    }

    /// Get current pattern statistics
    pub async fn pattern_stats(&self) -> PatternStats {
        let tracker = self.pattern_tracker.read().await;
        let manager = self.cluster_manager.read().await;

        PatternStats {
            total_clusters: manager.active_cluster_count(),
            active_patterns: tracker.active_pattern_count(),
        }
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of analyzing a log entry
pub struct AnalysisResult {
    pub entry: LogEntry,
    pub signature: String,
    pub is_new_pattern: bool,
    pub window_count: u32,
    pub incident: Option<crate::models::Incident>,
}

/// Pattern statistics
pub struct PatternStats {
    pub total_clusters: usize,
    pub active_patterns: usize,
}
