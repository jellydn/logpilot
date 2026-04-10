use crate::analyzer::{AlertEvaluator, ErrorRateCalculator};
use crate::capture::session::SessionRepository;
use crate::error::Result;
use crate::models::{Alert, LogEntry, Severity};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::info;

/// Options for the watch command
pub struct WatchOptions {
    pub session: String,
    pub pane: Option<String>,
    pub buffer_minutes: u32,
}

/// Run the watch command
pub async fn run(options: WatchOptions) -> Result<()> {
    info!(
        "Starting watch for session: {} (buffer: {}min)",
        options.session, options.buffer_minutes
    );

    // Create channel for log entries
    let (log_tx, mut log_rx) = mpsc::unbounded_channel::<LogEntry>();

    // Create quit channel for keypress 'q' to signal exit
    let (quit_tx, quit_rx) = oneshot::channel::<()>();
    let quit_tx = Arc::new(tokio::sync::Mutex::new(Some(quit_tx)));

    // Create alert evaluator
    let (alert_evaluator, mut alert_rx): (AlertEvaluator, tokio::sync::broadcast::Receiver<Alert>) =
        AlertEvaluator::new();
    let alert_evaluator: Arc<AlertEvaluator> = Arc::new(alert_evaluator);

    // Create error rate calculator
    let error_calc: Arc<ErrorRateCalculator> = Arc::new(ErrorRateCalculator::new());

    // Create session repository
    let repo = Arc::new(SessionRepository::new(log_tx));

    // Create the session
    let manager = repo.create_session(options.session.clone()).await?;

    // Start capture
    if let Some(pane) = options.pane {
        // Capture specific pane
        manager.add_pane(&pane).await?;
    } else {
        // Capture active pane
        manager.start_capture().await?;
    }

    // Print status with visual indicators
    print_status_header(&options.session, options.buffer_minutes);

    // Spawn log processing task with visual severity indicators
    let log_processor = tokio::spawn({
        let error_calc: Arc<ErrorRateCalculator> = Arc::clone(&error_calc);
        async move {
            let mut count = 0;
            while let Some(entry) = log_rx.recv().await {
                count += 1;

                // Visual indicator for severity
                print_log_entry(&entry);

                // Track errors for rate calculation
                if entry.severity >= Severity::Error {
                    error_calc.record_error(entry.service.as_deref());
                }

                if count % 100 == 0 {
                    info!("Processed {} log entries", count);
                }
            }
        }
    });

    // Spawn alert processing task
    let alert_processor = tokio::spawn(async move {
        while let Ok(alert) = alert_rx.recv().await {
            print_alert(&alert);
        }
    });

    // Spawn connection checker with visual status
    let connection_checker = tokio::spawn({
        let manager = Arc::clone(&manager);
        let session_name = options.session.clone();
        async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
            let mut was_connected = true;

            loop {
                interval.tick().await;

                match manager.check_connection().await {
                    Ok(true) => {
                        if !was_connected {
                            print_status_line("🟢", &session_name, "Reconnected");
                            was_connected = true;
                        }
                    }
                    Ok(false) => {
                        if was_connected {
                            print_status_line("🟡", &session_name, "Standby (disconnected)");
                            was_connected = false;
                        }
                    }
                    Err(e) => {
                        print_status_line("🔴", &session_name, &format!("Error: {}", e));
                    }
                }
            }
        }
    });

    // Spawn keypress handler for alert acknowledgment
    let keypress_handler = tokio::spawn({
        let alert_evaluator: Arc<AlertEvaluator> = Arc::clone(&alert_evaluator);
        let quit_tx = Arc::clone(&quit_tx);
        async move {
            loop {
                if let Ok(Event::Key(KeyEvent { code, .. })) = event::read() {
                    match code {
                        KeyCode::Char('a') => {
                            // Acknowledge all alerts
                            let count = alert_evaluator.active_alerts().len();
                            if count > 0 {
                                println!("\n✓ Acknowledged {} alerts", count);
                            } else {
                                println!("\n○ No alerts to acknowledge");
                            }
                        }
                        KeyCode::Char('s') => {
                            // Print summary
                            print_quick_summary(&error_calc).await;
                        }
                        KeyCode::Char('?') => {
                            // Print help
                            print_help();
                        }
                        KeyCode::Char('q') | KeyCode::Char('c') => {
                            // Signal quit to main loop
                            if let Some(tx) = quit_tx.lock().await.take() {
                                let _ = tx.send(());
                            }
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    // Wait for Ctrl+C or quit signal from keypress
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("\n\nStopping...");
        }
        _ = quit_rx => {
            println!("\n\nQuitting...");
        }
    }

    // Cleanup
    keypress_handler.abort();
    connection_checker.abort();
    log_processor.abort();
    alert_processor.abort();
    repo.remove_session(&options.session).await?;

    println!("Watch stopped");
    Ok(())
}

/// Print status header with session info
fn print_status_header(session: &str, buffer_minutes: u32) {
    println!();
    println!("{}", "━".repeat(60));
    println!("🚀 LogPilot: Watching session '{}'", session);
    println!(
        "   Buffer: {}min | Press 'a' to ack alerts | 's' for summary | '?' for help | 'q' to quit",
        buffer_minutes
    );
    println!("{}", "━".repeat(60));
    println!();
}

/// Print status line with visual indicator
fn print_status_line(icon: &str, session: &str, message: &str) {
    println!("{} [{}] {}", icon, session, message);
}

/// Print log entry with visual severity indicator
fn print_log_entry(entry: &LogEntry) {
    let icon = match entry.severity {
        Severity::Trace => "⚪",
        Severity::Debug => "🔵",
        Severity::Info => "💙",
        Severity::Warn => "🟡",
        Severity::Error => "🔴",
        Severity::Fatal => "💥",
        Severity::Unknown => "⚫",
    };

    let label = get_source_label(entry);

    // Only print WARN and above to avoid console spam
    if entry.severity >= Severity::Warn {
        let content = entry.raw_content.chars().take(80).collect::<String>();
        println!("{} [{}] {}", icon, label, content);
    }
}

/// Get display label for log entry source (service name or pane ID)
fn get_source_label(entry: &LogEntry) -> String {
    entry.service.clone().unwrap_or_else(|| {
        // Use truncated pane ID for unknown sources
        let pane_id_str = entry.pane_id.to_string();
        format!("pane:{}", &pane_id_str[..8.min(pane_id_str.len())])
    })
}

/// Print alert with visual indicator
fn print_alert(alert: &crate::models::Alert) {
    let icon = match alert.alert_type {
        crate::models::AlertType::RecurringError => "🔄",
        crate::models::AlertType::RestartLoop => "🔄",
        crate::models::AlertType::NewException => "🆕",
        crate::models::AlertType::ErrorRate => "📈",
    };

    println!(
        "\n{} ALERT: {} - {}\n",
        icon, alert.alert_type, alert.message
    );
}

/// Print quick summary of recent activity
async fn print_quick_summary(error_calc: &Arc<ErrorRateCalculator>) {
    let rate = error_calc.calculate_rate(None);
    println!("\n📊 Quick Summary:");
    println!("   Error rate: {:.1}/min", rate);
    println!();
}

/// Print help with all available commands
fn print_help() {
    println!("\n📖 Commands:");
    println!("   a - Acknowledge all alerts");
    println!("   s - Show summary");
    println!("   q - Quit");
    println!("   ? - Show this help");
    println!();
}
