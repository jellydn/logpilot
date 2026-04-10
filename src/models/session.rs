use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SessionStatus {
    Active,
    Stale,
    Disconnected,
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionStatus::Active => write!(f, "Active"),
            SessionStatus::Stale => write!(f, "Stale"),
            SessionStatus::Disconnected => write!(f, "Disconnected"),
        }
    }
}

impl SessionStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, SessionStatus::Active)
    }

    #[cfg(test)]
    pub fn can_reconnect(&self) -> bool {
        matches!(self, SessionStatus::Active | SessionStatus::Stale)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub name: String,
    pub tmux_socket: Option<String>,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub pane_ids: Vec<Uuid>,
}

impl Session {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            tmux_socket: None,
            status: SessionStatus::Active,
            created_at: now,
            last_seen: now,
            pane_ids: Vec::new(),
        }
    }

    pub fn add_pane(&mut self, pane_id: Uuid) {
        if !self.pane_ids.contains(&pane_id) {
            self.pane_ids.push(pane_id);
        }
    }

    #[cfg(test)]
    pub fn remove_pane(&mut self, pane_id: &Uuid) {
        self.pane_ids.retain(|&id| id != *pane_id);
    }

    pub fn mark_active(&mut self) {
        self.status = SessionStatus::Active;
        self.last_seen = Utc::now();
    }

    pub fn mark_stale(&mut self) {
        self.status = SessionStatus::Stale;
    }

    pub fn mark_disconnected(&mut self) {
        self.status = SessionStatus::Disconnected;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("test-session");
        assert_eq!(session.name, "test-session");
        assert!(session.status.is_active());
        assert!(session.pane_ids.is_empty());
    }

    #[test]
    fn test_session_status_transitions() {
        let mut session = Session::new("test");
        session.mark_stale();
        assert!(!session.status.is_active());
        assert!(session.status.can_reconnect());

        session.mark_disconnected();
        assert!(!session.status.can_reconnect());

        session.mark_active();
        assert!(session.status.is_active());
    }

    #[test]
    fn test_session_pane_management() {
        let mut session = Session::new("test");
        let pane_id = Uuid::new_v4();

        session.add_pane(pane_id);
        assert_eq!(session.pane_ids.len(), 1);

        // Duplicate add should not increase count
        session.add_pane(pane_id);
        assert_eq!(session.pane_ids.len(), 1);

        session.remove_pane(&pane_id);
        assert!(session.pane_ids.is_empty());
    }
}
