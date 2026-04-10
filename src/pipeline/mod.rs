//! Log processing pipeline
//!
//! Producer-consumer architecture using tokio channels:
//! Capture -> Parser -> Deduplicator -> Cluster -> Analyzer
#![allow(dead_code)] // Infrastructure not yet wired to CLI

pub mod cluster;
pub mod dedup;
pub mod formats;
pub mod parser;

use crate::models::LogEntry;
use tokio::sync::mpsc;

/// Pipeline orchestrator that wires components together
pub struct Pipeline {
    entry_tx: mpsc::UnboundedSender<LogEntry>,
}

impl Pipeline {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<LogEntry>) {
        let (entry_tx, entry_rx) = mpsc::unbounded_channel::<LogEntry>();

        (Self { entry_tx }, entry_rx)
    }

    pub fn entry_sender(&self) -> mpsc::UnboundedSender<LogEntry> {
        self.entry_tx.clone()
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        let (tx, _rx) = mpsc::unbounded_channel::<LogEntry>();
        Self { entry_tx: tx }
    }
}
