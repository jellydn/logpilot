use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PaneStatus {
    Capturing,
    Paused,
    Error,
}

impl fmt::Display for PaneStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaneStatus::Capturing => write!(f, "Capturing"),
            PaneStatus::Paused => write!(f, "Paused"),
            PaneStatus::Error => write!(f, "Error"),
        }
    }
}

impl PaneStatus {
    #[cfg(test)]
    pub fn is_capturing(&self) -> bool {
        matches!(self, PaneStatus::Capturing)
    }

    #[cfg(test)]
    pub fn is_active(&self) -> bool {
        matches!(self, PaneStatus::Capturing | PaneStatus::Paused)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pane {
    pub id: Uuid,
    pub session_id: Uuid,
    pub tmux_id: String,
    pub status: PaneStatus,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

impl Pane {
    pub fn new(session_id: Uuid, tmux_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            session_id,
            tmux_id: tmux_id.into(),
            status: PaneStatus::Capturing,
            created_at: now,
            last_activity: now,
        }
    }

    #[cfg(test)]
    pub fn mark_capturing(&mut self) {
        self.status = PaneStatus::Capturing;
        self.last_activity = Utc::now();
    }

    #[cfg(test)]
    pub fn mark_paused(&mut self) {
        self.status = PaneStatus::Paused;
    }

    #[cfg(test)]
    pub fn mark_error(&mut self) {
        self.status = PaneStatus::Error;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pane_creation() {
        let session_id = Uuid::new_v4();
        let pane = Pane::new(session_id, "test-session:1.0");

        assert_eq!(pane.tmux_id, "test-session:1.0");
        assert_eq!(pane.session_id, session_id);
        assert!(pane.status.is_capturing());
    }

    #[test]
    fn test_pane_status_transitions() {
        let session_id = Uuid::new_v4();
        let mut pane = Pane::new(session_id, "test:1.0");

        pane.mark_paused();
        assert!(pane.status.is_active());
        assert!(!pane.status.is_capturing());

        pane.mark_error();
        assert!(!pane.status.is_active());

        pane.mark_capturing();
        assert!(pane.status.is_capturing());
    }
}
