use super::severity::Severity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum IncidentStatus {
    Active,
    Mitigating,
    Resolved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: Uuid,
    pub title: String,
    pub severity: Severity,
    pub status: IncidentStatus,
    pub started_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub pattern_ids: Vec<Uuid>,
    pub affected_services: Vec<String>,
    pub entry_count: u64,
}

impl Incident {
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            severity: Severity::Warn,
            status: IncidentStatus::Active,
            started_at: now,
            resolved_at: None,
            pattern_ids: Vec::new(),
            affected_services: Vec::new(),
            entry_count: 0,
        }
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    #[cfg(test)]
    pub fn add_pattern(&mut self, pattern_id: Uuid) {
        if !self.pattern_ids.contains(&pattern_id) {
            self.pattern_ids.push(pattern_id);
        }
    }

    #[cfg(test)]
    pub fn add_service(&mut self, service: impl Into<String>) {
        let service = service.into();
        if !self.affected_services.contains(&service) {
            self.affected_services.push(service);
        }
    }

    #[cfg(test)]
    pub fn mark_mitigating(&mut self) {
        self.status = IncidentStatus::Mitigating;
    }

    #[cfg(test)]
    pub fn resolve(&mut self) {
        self.status = IncidentStatus::Resolved;
        self.resolved_at = Some(Utc::now());
    }

    #[cfg(test)]
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            IncidentStatus::Active | IncidentStatus::Mitigating
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incident_creation() {
        let incident = Incident::new("Connection failures");
        assert_eq!(incident.title, "Connection failures");
        assert!(incident.is_active());
        assert!(incident.resolved_at.is_none());
    }

    #[test]
    fn test_incident_lifecycle() {
        let mut incident = Incident::new("Test incident");

        incident.mark_mitigating();
        assert!(matches!(incident.status, IncidentStatus::Mitigating));

        incident.resolve();
        assert!(!incident.is_active());
        assert!(incident.resolved_at.is_some());
    }

    #[test]
    fn test_pattern_and_service_management() {
        let mut incident = Incident::new("Test");
        let pattern_id = Uuid::new_v4();

        incident.add_pattern(pattern_id);
        incident.add_pattern(pattern_id); // Duplicate should be ignored
        assert_eq!(incident.pattern_ids.len(), 1);

        incident.add_service("api");
        incident.add_service("api"); // Duplicate should be ignored
        assert_eq!(incident.affected_services.len(), 1);
    }
}
