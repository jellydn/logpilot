//! Alert triggering and management

use crate::models::{Alert, AlertStatus, AlertType, Pattern, Severity};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Evaluates conditions and triggers alerts
pub struct AlertEvaluator {
    /// Active alerts
    alerts: DashMap<Uuid, Alert>,
    /// Broadcast channel for alert notifications
    alert_tx: Arc<broadcast::Sender<Alert>>,
    /// Thresholds
    error_rate_threshold: f64,
    recurring_error_threshold: u32,
}

impl AlertEvaluator {
    pub fn new() -> (Self, broadcast::Receiver<Alert>) {
        let (alert_tx, alert_rx) = broadcast::channel(100);

        let evaluator = Self {
            alerts: DashMap::new(),
            alert_tx: Arc::new(alert_tx),
            error_rate_threshold: 10.0, // 10 errors/min
            recurring_error_threshold: 5,
        };

        (evaluator, alert_rx)
    }

    /// Evaluate error rate and trigger alert if over threshold
    pub fn check_error_rate(
        &self,
        errors_per_minute: f64,
        _service: Option<&str>,
    ) -> Option<Alert> {
        if errors_per_minute >= self.error_rate_threshold {
            self.create_alert(
                AlertType::ErrorRate,
                format!("Error rate {:.1}/min exceeds threshold", errors_per_minute),
                Severity::Warn,
                None,
                None,
                Some(errors_per_minute),
            )
        } else {
            None
        }
    }

    /// Check for recurring error pattern
    pub fn check_recurring_error(&self, pattern: &Pattern) -> Option<Alert> {
        if pattern.window_count >= self.recurring_error_threshold {
            self.create_alert(
                AlertType::RecurringError,
                format!(
                    "Recurring error: {} occurrences in 60s",
                    pattern.window_count
                ),
                pattern.severity,
                None,
                Some(pattern.id),
                Some(pattern.window_count as f64),
            )
        } else {
            None
        }
    }

    /// Check for new exception
    pub fn check_new_exception(&self, pattern: &Pattern, is_new: bool) -> Option<Alert> {
        if is_new && pattern.severity >= Severity::Error {
            self.create_alert(
                AlertType::NewException,
                format!(
                    "New unseen exception: {}",
                    &pattern.signature[..20.min(pattern.signature.len())]
                ),
                pattern.severity,
                None,
                Some(pattern.id),
                Some(1.0),
            )
        } else {
            None
        }
    }

    /// Check for restart loop
    pub fn check_restart_loop(&self, service: &str, is_looping: bool) -> Option<Alert> {
        if is_looping {
            // Check if we already have an active alert for this
            let existing = self.alerts.iter().find(|entry| {
                let alert = entry.value();
                alert.alert_type == AlertType::RestartLoop
                    && alert.status == AlertStatus::Active
                    && alert.message.contains(service)
            });

            if existing.is_none() {
                return self.create_alert(
                    AlertType::RestartLoop,
                    format!("Service restart loop detected: {}", service),
                    Severity::Error,
                    None,
                    None,
                    Some(1.0),
                );
            }
        }
        None
    }

    fn create_alert(
        &self,
        alert_type: AlertType,
        message: String,
        severity: Severity,
        incident_id: Option<Uuid>,
        pattern_id: Option<Uuid>,
        current_value: Option<f64>,
    ) -> Option<Alert> {
        // Check for deduplication
        if let Some(alert_id) = self.find_duplicate(&alert_type, incident_id, pattern_id) {
            // Update existing alert
            if let Some(mut alert) = self.alerts.get_mut(&alert_id) {
                if let Some(val) = current_value {
                    alert.current_value = val;
                }
                return Some(alert.clone());
            }
        }

        // Create new alert
        let mut alert = Alert::new(alert_type, message, current_value.unwrap_or(1.0));
        alert.severity = severity;
        if let Some(id) = incident_id {
            alert.incident_id = Some(id);
        }
        if let Some(id) = pattern_id {
            alert.pattern_id = Some(id);
        }

        let id = alert.id;
        self.alerts.insert(id, alert.clone());

        // Broadcast alert
        let _ = self.alert_tx.send(alert.clone());

        Some(alert)
    }

    fn find_duplicate(
        &self,
        alert_type: &AlertType,
        incident_id: Option<Uuid>,
        pattern_id: Option<Uuid>,
    ) -> Option<Uuid> {
        for entry in self.alerts.iter() {
            let alert = entry.value();
            if alert.status != AlertStatus::Active {
                continue;
            }
            if &alert.alert_type != alert_type {
                continue;
            }
            if alert.incident_id == incident_id && alert.pattern_id == pattern_id {
                return Some(*entry.key());
            }
        }
        None
    }

    /// Acknowledge an alert
    pub fn acknowledge(&self, alert_id: Uuid) -> bool {
        if let Some(mut alert) = self.alerts.get_mut(&alert_id) {
            alert.acknowledge();
            true
        } else {
            false
        }
    }

    /// Resolve an alert
    pub fn resolve(&self, alert_id: Uuid) -> bool {
        if let Some(mut alert) = self.alerts.get_mut(&alert_id) {
            alert.resolve();
            true
        } else {
            false
        }
    }

    /// Get all active alerts
    pub fn active_alerts(&self) -> Vec<Alert> {
        self.alerts
            .iter()
            .filter(|entry| entry.value().status == AlertStatus::Active)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get alert count
    pub fn count(&self) -> usize {
        self.alerts.len()
    }
}

impl Default for AlertEvaluator {
    fn default() -> Self {
        let (evaluator, _) = Self::new();
        evaluator
    }
}

/// Repository for alert storage
pub struct AlertRepository {
    alerts: DashMap<Uuid, Alert>,
}

impl AlertRepository {
    pub fn new() -> Self {
        Self {
            alerts: DashMap::new(),
        }
    }

    pub fn store(&self, alert: Alert) {
        self.alerts.insert(alert.id, alert);
    }

    pub fn get(&self, id: Uuid) -> Option<Alert> {
        self.alerts.get(&id).map(|a| a.clone())
    }

    pub fn list_active(&self) -> Vec<Alert> {
        self.alerts
            .iter()
            .filter(|e| e.value().status == AlertStatus::Active)
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn list_all(&self) -> Vec<Alert> {
        self.alerts.iter().map(|e| e.value().clone()).collect()
    }
}

impl Default for AlertRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculates error rates over sliding windows
pub struct ErrorRateCalculator {
    /// Recent errors with timestamps (service -> list of timestamps)
    errors: DashMap<String, Vec<chrono::DateTime<chrono::Utc>>>,
    /// Window duration in minutes
    window_minutes: i64,
}

impl ErrorRateCalculator {
    pub fn new() -> Self {
        Self {
            errors: DashMap::new(),
            window_minutes: 1,
        }
    }

    /// Record an error occurrence
    pub fn record_error(&self, service: Option<&str>) {
        let key = service.unwrap_or("_global").to_string();
        let now = chrono::Utc::now();

        self.errors.entry(key).or_default().push(now);
    }

    /// Calculate current error rate (errors per minute)
    pub fn calculate_rate(&self, service: Option<&str>) -> f64 {
        let key = service.unwrap_or("_global").to_string();
        let now = chrono::Utc::now();
        let cutoff = now - chrono::Duration::minutes(self.window_minutes);

        if let Some(timestamps) = self.errors.get(&key) {
            let recent: Vec<_> = timestamps.iter().filter(|ts| **ts >= cutoff).collect();
            recent.len() as f64
        } else {
            0.0
        }
    }

    /// Clean up old entries
    pub fn cleanup(&self) {
        let now = chrono::Utc::now();
        let cutoff = now - chrono::Duration::minutes(self.window_minutes * 2);

        for mut entry in self.errors.iter_mut() {
            entry.value_mut().retain(|ts| *ts >= cutoff);
        }
    }
}

impl Default for ErrorRateCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Pattern;

    fn create_test_pattern(signature: &str, window_count: u32, severity: Severity) -> Pattern {
        let mut pattern = Pattern::new(signature).with_severity(severity);
        pattern.window_count = window_count;
        pattern
    }

    #[test]
    fn test_recurring_error_alert() {
        let (evaluator, _rx) = AlertEvaluator::new();
        let pattern = create_test_pattern("test-sig", 5, Severity::Error);

        let alert = evaluator.check_recurring_error(&pattern);
        assert!(alert.is_some());

        let alert = alert.unwrap();
        assert_eq!(alert.alert_type, AlertType::RecurringError);
        assert_eq!(alert.status, AlertStatus::Active);
    }

    #[test]
    fn test_alert_deduplication() {
        let (evaluator, _rx) = AlertEvaluator::new();
        let pattern = create_test_pattern("test-sig", 5, Severity::Error);

        // First alert
        let alert1 = evaluator.check_recurring_error(&pattern).unwrap();

        // Second alert for same pattern should update existing
        let mut pattern2 = create_test_pattern("test-sig", 6, Severity::Error);
        pattern2.id = pattern.id; // Same pattern ID
        let alert2 = evaluator.check_recurring_error(&pattern2).unwrap();

        assert_eq!(alert1.id, alert2.id);
    }

    #[test]
    fn test_error_rate_calculation() {
        let calc = ErrorRateCalculator::new();

        // Record 5 errors
        for _ in 0..5 {
            calc.record_error(Some("test-service"));
        }

        let rate = calc.calculate_rate(Some("test-service"));
        assert_eq!(rate, 5.0);

        // Global rate should be 0 (no global errors recorded)
        let global_rate = calc.calculate_rate(None);
        assert_eq!(global_rate, 0.0);
    }
}
