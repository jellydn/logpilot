//! Buffer module for log storage and persistence
//!
//! Combines in-memory ring buffer with SQLite persistence for high-severity events
#![allow(dead_code)] // Infrastructure not yet wired to CLI

pub mod manager;
pub mod persistence;
pub mod ring;
