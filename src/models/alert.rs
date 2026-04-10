use super::severity::Severity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum AlertType {
    RecurringError,
    RestartLoop,
    NewException,
    ErrorRate,
}

impl fmt::Display for AlertType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlertType::RecurringError => write!(f, "Recurring Error"),
            AlertType::RestartLoop => write!(f, "Restart Loop"),
            AlertType::NewException => write!(f, "New Exception"),
            AlertType::ErrorRate => write!(f, "Error Rate"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum AlertStatus {
    Active,
    Acknowledged,
    Resolved,
}

impl fmt::Display for AlertStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlertStatus::Active => write!(f, "Active"),
            AlertStatus::Acknowledged => write!(f, "Acknowledged"),
            AlertStatus::Resolved => write!(f, "Resolved"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: Uuid,
    pub alert_type: AlertType,
    pub incident_id: Option<Uuid>,
    pub pattern_id: Option<Uuid>,
    pub threshold: Option<f64>,
    pub current_value: f64,
    pub triggered_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub status: AlertStatus,
    pub message: String,
    pub severity: Severity,
}

impl Alert {
    pub fn new(alert_type: AlertType, message: impl Into<String>, current_value: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            alert_type,
            incident_id: None,
            pattern_id: None,
            threshold: None,
            current_value,
            triggered_at: Utc::now(),
            acknowledged_at: None,
            status: AlertStatus::Active,
            message: message.into(),
            severity: Severity::Warn,
        }
    }

    #[cfg(test)]
    pub fn with_incident(mut self, incident_id: Uuid) -> Self {
        self.incident_id = Some(incident_id);
        self
    }

    #[cfg(test)]
    pub fn dedup_key(&self) -> String {
        format!(
            "{:?}:{}:{}",
            self.alert_type,
            self.incident_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            self.pattern_id.map(|id| id.to_string()).unwrap_or_default()
        )
    }

    pub fn acknowledge(&mut self) {
        if self.status == AlertStatus::Active {
            self.status = AlertStatus::Acknowledged;
            self.acknowledged_at = Some(Utc::now());
        }
    }

    pub fn resolve(&mut self) {
        self.status = AlertStatus::Resolved;
    }

    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        matches!(self.status, AlertStatus::Active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_creation() {
        let alert = Alert::new(AlertType::RecurringError, "Pattern detected", 5.0);
        assert!(alert.is_active());
        assert_eq!(alert.current_value, 5.0);
    }

    #[test]
    fn test_alert_lifecycle() {
        let mut alert = Alert::new(AlertType::ErrorRate, "High error rate", 15.0);

        alert.acknowledge();
        assert!(!alert.is_active());
        assert!(alert.acknowledged_at.is_some());

        alert.resolve();
        assert!(matches!(alert.status, AlertStatus::Resolved));
    }

    #[test]
    fn test_dedup_key() {
        let alert1 =
            Alert::new(AlertType::RecurringError, "Test", 1.0).with_incident(Uuid::new_v4());
        let alert2 = Alert::new(AlertType::RecurringError, "Test", 1.0)
            .with_incident(alert1.incident_id.unwrap());

        assert_eq!(alert1.dedup_key(), alert2.dedup_key());
    }
}
