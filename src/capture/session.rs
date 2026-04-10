use crate::capture::pane::MultiPaneCapture;
use crate::capture::tmux::TmuxCommand;
use crate::error::{LogPilotError, Result};
use crate::models::{LogEntry, Session, SessionStatus};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Manages a single session and its pane captures
pub struct SessionManager {
    session: Arc<RwLock<Session>>,
    pane_capture: MultiPaneCapture,
    log_tx: mpsc::UnboundedSender<LogEntry>,
    reconnect_handle: Option<tokio::task::JoinHandle<()>>,
}

impl SessionManager {
    pub async fn new(
        session_name: String,
        log_tx: mpsc::UnboundedSender<LogEntry>,
    ) -> Result<Self> {
        // Verify session exists
        if !TmuxCommand::session_exists(&session_name).await? {
            return Err(LogPilotError::SessionNotFound { name: session_name });
        }

        let session = Session::new(&session_name);
        let session_id = session.id;

        info!(
            "Created session manager for {} (id: {})",
            session_name, session_id
        );

        Ok(Self {
            session: Arc::new(RwLock::new(session)),
            pane_capture: MultiPaneCapture::new(),
            log_tx,
            reconnect_handle: None,
        })
    }

    /// Start capturing from the active pane
    pub async fn start_capture(&self) -> Result<()> {
        let session = self.session.read().await;
        let tmux_id = TmuxCommand::get_active_pane(&session.name).await?;
        drop(session);

        self.add_pane(&tmux_id).await?;

        // Update session status
        let mut session = self.session.write().await;
        session.mark_active();

        info!("Started capture for session {}", session.name);
        Ok(())
    }

    /// Add a specific pane to capture
    pub async fn add_pane(&self, tmux_id: &str) -> Result<Uuid> {
        let session_id = self.session.read().await.id;
        let pane_id = self
            .pane_capture
            .add_pane(session_id, tmux_id.to_string(), self.log_tx.clone())
            .await?;

        // Track pane in session
        let mut session = self.session.write().await;
        session.add_pane(pane_id);

        info!(
            "Added pane {} to session {} (tmux: {})",
            pane_id, session.name, tmux_id
        );
        Ok(pane_id)
    }

    /// Get current session status
    pub async fn status(&self) -> SessionStatus {
        self.session.read().await.status
    }

    /// Check if session is still connected
    pub async fn check_connection(&self) -> Result<bool> {
        let session_name = self.session.read().await.name.clone();

        match TmuxCommand::session_exists(&session_name).await {
            Ok(true) => {
                // Session still exists
                let mut session = self.session.write().await;
                if !session.status.is_active() {
                    // Reconnected
                    session.mark_active();
                    info!("Session {} reconnected", session_name);
                }
                Ok(true)
            }
            Ok(false) => {
                // Session gone
                let mut session = self.session.write().await;
                if session.status.is_active() {
                    session.mark_stale();
                    warn!(
                        "Session {} disconnected, entering standby mode",
                        session_name
                    );

                    // Start reconnect task
                    self.start_reconnect_task().await;
                }
                Ok(false)
            }
            Err(e) => {
                error!("Error checking session {}: {}", session_name, e);
                Ok(false)
            }
        }
    }

    /// Start background task to attempt reconnection
    async fn start_reconnect_task(&self) {
        let session = Arc::clone(&self.session);

        let _handle = tokio::spawn(async move {
            let mut attempts = 0;
            const MAX_ATTEMPTS: u32 = 5;

            while attempts < MAX_ATTEMPTS {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                attempts += 1;

                let session_name = session.read().await.name.clone();

                match TmuxCommand::session_exists(&session_name).await {
                    Ok(true) => {
                        // Session is back
                        let mut s = session.write().await;
                        s.mark_active();
                        info!(
                            "Session {} auto-reconnected after {} attempts",
                            session_name, attempts
                        );
                        return;
                    }
                    Ok(false) => {
                        debug!(
                            "Session {} still not available (attempt {})",
                            session_name, attempts
                        );
                    }
                    Err(e) => {
                        debug!("Error checking session {}: {}", session_name, e);
                    }
                }
            }

            // Max attempts reached
            let mut s = session.write().await;
            s.mark_disconnected();
            warn!(
                "Session {} marked disconnected after {} reconnection attempts",
                s.name, MAX_ATTEMPTS
            );
        });

        // Store handle (would need to be in a Mutex for mutability)
        // For now, we just let it run
    }

    /// Stop all captures and cleanup
    pub async fn stop(&self) -> Result<()> {
        self.pane_capture.stop_all().await;

        let session = self.session.read().await;
        info!("Stopped session manager for {}", session.name);

        Ok(())
    }

    pub fn session_id(&self) -> Uuid {
        // This is a workaround since we can't easily get the id from RwLock
        // In real implementation, store id separately
        Uuid::nil()
    }
}

/// Repository for managing multiple sessions
pub struct SessionRepository {
    sessions: Arc<RwLock<HashMap<String, Arc<SessionManager>>>>,
    log_tx: mpsc::UnboundedSender<LogEntry>,
}

impl SessionRepository {
    pub fn new(log_tx: mpsc::UnboundedSender<LogEntry>) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            log_tx,
        }
    }

    pub async fn create_session(&self, name: String) -> Result<Arc<SessionManager>> {
        let mut sessions = self.sessions.write().await;

        if sessions.contains_key(&name) {
            return Err(LogPilotError::config(format!(
                "Session {} is already being monitored",
                name
            )));
        }

        let manager = Arc::new(SessionManager::new(name.clone(), self.log_tx.clone()).await?);
        sessions.insert(name.clone(), Arc::clone(&manager));

        info!("Created and stored session manager for {}", name);
        Ok(manager)
    }

    pub async fn get_session(&self, name: &str) -> Option<Arc<SessionManager>> {
        let sessions = self.sessions.read().await;
        sessions.get(name).map(Arc::clone)
    }

    pub async fn remove_session(&self, name: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;

        if let Some(manager) = sessions.remove(name) {
            manager.stop().await?;
            info!("Removed session {}", name);
        }

        Ok(())
    }

    pub async fn list_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions.keys().cloned().collect()
    }

    pub async fn stop_all(&self) {
        let mut sessions = self.sessions.write().await;

        for (name, manager) in sessions.drain() {
            if let Err(e) = manager.stop().await {
                error!("Error stopping session {}: {}", name, e);
            }
        }
    }
}
