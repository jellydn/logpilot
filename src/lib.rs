pub mod analyzer;
pub mod buffer;
pub mod capture;
pub mod cli;
pub mod error;
pub mod mcp;
pub mod models;
pub mod observability;
pub mod pipeline;

pub use analyzer::Analyzer;
pub use error::{LogPilotError, Result};

pub use pipeline::Pipeline;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration loaded from TOML file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub buffer: BufferConfig,
    pub patterns: PatternConfig,
    pub alerts: AlertConfig,
    pub mcp: McpConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferConfig {
    pub duration_minutes: u32,
    pub max_memory_mb: u32,
    pub persist_severity: Vec<String>,
    pub persist_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternConfig {
    pub custom_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub recurring_error_window_seconds: u64,
    pub recurring_error_threshold: u32,
    pub restart_loop_window_seconds: u64,
    pub error_rate_threshold_per_minute: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub enabled: bool,
    pub transport: String,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .map(|d| d.join("logpilot"))
            .unwrap_or_else(|| PathBuf::from(".logpilot"));

        Self {
            buffer: BufferConfig {
                duration_minutes: 30,
                max_memory_mb: 100,
                persist_severity: vec!["ERROR".to_string(), "FATAL".to_string()],
                persist_path: data_dir,
            },
            patterns: PatternConfig {
                custom_patterns: Vec::new(),
            },
            alerts: AlertConfig {
                recurring_error_window_seconds: 60,
                recurring_error_threshold: 5,
                restart_loop_window_seconds: 30,
                error_rate_threshold_per_minute: 10,
            },
            mcp: McpConfig {
                enabled: true,
                transport: "stdio".to_string(),
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = dirs::config_dir()
            .map(|d| d.join("logpilot").join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("logpilot.toml"));

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).map_err(|e| {
                error::LogPilotError::config(format!("Failed to read config: {}", e))
            })?;
            let config: Config = toml::from_str(&content).map_err(|e| {
                error::LogPilotError::config(format!("Failed to parse config: {}", e))
            })?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }
}
