use crate::capture::tmux::TmuxCommand;
use crate::error::{LogPilotError, Result};
use crate::models::{LogEntry, Pane};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tracing::{debug, error, info};
use uuid::Uuid;

/// Manages capture for a single tmux pane
pub struct PaneCapture {
    pub pane: Pane,
    pub session_id: Uuid,
    fifo_path: PathBuf,
    tx: mpsc::UnboundedSender<LogEntry>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl PaneCapture {
    pub async fn start(
        session_id: Uuid,
        tmux_id: String,
        tx: mpsc::UnboundedSender<LogEntry>,
    ) -> Result<(Self, tokio::task::JoinHandle<()>)> {
        let pane = Pane::new(session_id, &tmux_id);
        let pane_id = pane.id;

        // Create FIFO (named pipe) for this pane
        let fifo_path =
            std::env::temp_dir().join(format!("logpilot-fifo-{}-{}", pane_id, std::process::id()));

        // Create the FIFO
        tokio::process::Command::new("mkfifo")
            .arg(&fifo_path)
            .output()
            .await
            .map_err(LogPilotError::Io)?;

        // Start tmux pipe-pane
        TmuxCommand::pipe_pane(&tmux_id, &fifo_path.to_string_lossy()).await?;

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        // Spawn capture task
        let handle = tokio::spawn(Self::capture_loop(
            pane_id,
            fifo_path.clone(),
            tx.clone(),
            shutdown_rx,
        ));

        let capture = Self {
            pane,
            session_id,
            fifo_path,
            tx,
            shutdown_tx: Some(shutdown_tx),
        };

        info!("Started capture for pane {} (tmux: {})", pane_id, tmux_id);

        Ok((capture, handle))
    }

    async fn capture_loop(
        pane_id: Uuid,
        fifo_path: PathBuf,
        tx: mpsc::UnboundedSender<LogEntry>,
        mut shutdown_rx: tokio::sync::oneshot::Receiver<()>,
    ) {
        let mut sequence: u64 = 0;
        let mut shutdown_received = false;

        loop {
            // Check if shutdown was requested before reopening FIFO
            if shutdown_received {
                break;
            }

            // Open the FIFO for reading
            let file = match File::open(&fifo_path).await {
                Ok(f) => f,
                Err(e) => {
                    error!("Failed to open FIFO for pane {}: {}", pane_id, e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue;
                }
            };

            let reader = BufReader::new(file);
            let mut lines = reader.lines();

            let should_break = loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = &mut shutdown_rx => {
                        info!("Shutdown signal received for pane {}", pane_id);
                        shutdown_received = true;
                        break true;
                    }

                    // Read line from FIFO
                    line_result = lines.next_line() => {
                        match line_result {
                            Ok(Some(line)) => {
                                sequence += 1;
                                let entry = LogEntry::new(
                                    pane_id,
                                    sequence,
                                    chrono::Utc::now(),
                                    line,
                                );

                                if let Err(e) = tx.send(entry) {
                                    error!("Failed to send log entry: {}", e);
                                    break true;
                                }
                            }
                            Ok(None) => {
                                // EOF - tmux might have closed the pipe
                                debug!("FIFO closed for pane {}, will retry", pane_id);
                                break false;
                            }
                            Err(e) => {
                                error!("Error reading from FIFO: {}", e);
                                break false;
                            }
                        }
                    }
                }
            };

            if should_break {
                break;
            }

            // Brief pause before retrying FIFO open
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Cleanup
        let _ = tokio::fs::remove_file(&fifo_path).await;
    }

    pub async fn stop(mut self) -> Result<()> {
        // Stop the tmux pipe-pane
        TmuxCommand::stop_pipe(&self.pane.tmux_id).await?;

        // Send shutdown signal to capture loop
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Cleanup FIFO
        let _ = tokio::fs::remove_file(&self.fifo_path).await;

        info!("Stopped capture for pane {}", self.pane.id);
        Ok(())
    }

    pub fn pane_id(&self) -> Uuid {
        self.pane.id
    }
}

/// Manages multiple pane captures for a session
pub struct MultiPaneCapture {
    captures: dashmap::DashMap<Uuid, (PaneCapture, tokio::task::JoinHandle<()>)>,
}

impl MultiPaneCapture {
    pub fn new() -> Self {
        Self {
            captures: dashmap::DashMap::new(),
        }
    }

    pub async fn add_pane(
        &self,
        session_id: Uuid,
        tmux_id: String,
        tx: mpsc::UnboundedSender<LogEntry>,
    ) -> Result<Uuid> {
        let (capture, handle) = PaneCapture::start(session_id, tmux_id, tx).await?;
        let pane_id = capture.pane_id();

        self.captures.insert(pane_id, (capture, handle));
        Ok(pane_id)
    }

    pub async fn remove_pane(&self, pane_id: Uuid) -> Result<()> {
        if let Some((_, (capture, _))) = self.captures.remove(&pane_id) {
            capture.stop().await?;
        }
        Ok(())
    }

    pub fn pane_count(&self) -> usize {
        self.captures.len()
    }

    pub async fn stop_all(&self) {
        // Signal shutdown to all captures
        for mut entry in self.captures.iter_mut() {
            let (capture, _) = entry.value_mut();
            if let Some(tx) = capture.shutdown_tx.take() {
                let _ = tx.send(());
            }
        }
        self.captures.clear();
    }
}

impl Default for MultiPaneCapture {
    fn default() -> Self {
        Self::new()
    }
}
