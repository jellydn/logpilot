use thiserror::Error;

#[derive(Error, Debug)]
pub enum LogPilotError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Tmux error: {message}")]
    Tmux { message: String },

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Database operation failed: {message}")]
    DatabaseOp { message: String },

    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Session not found: {name}")]
    SessionNotFound { name: String },
}

impl LogPilotError {
    pub fn tmux(message: impl Into<String>) -> Self {
        LogPilotError::Tmux {
            message: message.into(),
        }
    }

    pub fn config(message: impl Into<String>) -> Self {
        LogPilotError::Config {
            message: message.into(),
        }
    }

    pub fn db_op(message: impl Into<String>) -> Self {
        LogPilotError::DatabaseOp {
            message: message.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, LogPilotError>;
