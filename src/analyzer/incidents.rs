//! Incident detection and management

use crate::models::{Incident, IncidentStatus, Severity};
use dashmap::DashMap;
use std::collections::HashSet;
use uuid::Uuid;

/// Auto-creates incidents from pattern spikes
pub struct IncidentDetector {
    /// Active incidents
    incidents: DashMap<Uuid, Incident>,
    /// Map of pattern signatures to incident IDs
    pattern_to_incident: DashMap<String, Uuid>,
    /// Minimum entries to create incident without pattern
    min_unpatterned_errors: u32,
    /// Spike threshold for auto-creation (unused but planned)
    spike_threshold: u32,
}

impl IncidentDetector {
    pub fn new() -> Self {
        Self {
            incidents: DashMap::new(),
            pattern_to_incident: DashMap::new(),
            min_unpatterned_errors: 10,
            spike_threshold: 20, // 20 occurrences in window = spike
        }
    }

    /// Create an incident from a detected pattern
    pub async fn create_incident(
        &self,
        signature: &str,
        entry: &crate::models::LogEntry,
        window_count: u32,
    ) -> Incident {
        // Check if there's already an incident for this pattern
        if let Some(existing_id) = self.pattern_to_incident.get(signature) {
            let id = *existing_id;
            drop(existing_id);

            // Update existing incident
            if let Some(mut incident) = self.incidents.get_mut(&id) {
                incident.entry_count += 1;
                if entry.severity > incident.severity {
                    incident.severity = entry.severity;
                }
                return incident.clone();
            }
        }

        // Create new incident
        let incident = Incident::new(format!(
            "Pattern spike: {} ({} occurrences)",
            &signature[..8.min(signature.len())],
            window_count
        ))
        .with_severity(entry.severity);

        let id = incident.id;

        // Store pattern mapping
        self.pattern_to_incident.insert(signature.to_string(), id);

        // Store incident
        self.incidents.insert(id, incident.clone());

        incident
    }

    /// Create incident from unpatterned error spike
    pub fn create_from_errors(&self, errors: &[crate::models::LogEntry]) -> Option<Incident> {
        if errors.len() < self.min_unpatterned_errors as usize {
            return None;
        }

        let severity = errors
            .iter()
            .map(|e| e.severity)
            .max()
            .unwrap_or(Severity::Error);

        let _affected_services: HashSet<String> =
            errors.iter().filter_map(|e| e.service.clone()).collect();

        let incident = Incident::new(format!("Unpatterned error spike: {} errors", errors.len()))
            .with_severity(severity);

        let id = incident.id;
        self.incidents.insert(id, incident.clone());

        Some(incident)
    }

    /// Get an incident by ID
    pub fn get(&self, id: Uuid) -> Option<Incident> {
        self.incidents.get(&id).map(|i| i.clone())
    }

    /// Get all active incidents
    pub fn active_incidents(&self) -> Vec<Incident> {
        self.incidents
            .iter()
            .filter(|entry| entry.status == IncidentStatus::Active)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Resolve an incident
    pub fn resolve(&self, id: Uuid) -> bool {
        if let Some(mut incident) = self.incidents.get_mut(&id) {
            incident.status = IncidentStatus::Resolved;
            incident.resolved_at = Some(chrono::Utc::now());
            true
        } else {
            false
        }
    }

    /// Get incident count
    pub fn count(&self) -> usize {
        self.incidents.len()
    }
}

impl Default for IncidentDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Repository for incident storage and queries
pub struct IncidentRepository {
    incidents: DashMap<Uuid, Incident>,
}

impl IncidentRepository {
    pub fn new() -> Self {
        Self {
            incidents: DashMap::new(),
        }
    }

    pub fn store(&self, incident: Incident) {
        self.incidents.insert(incident.id, incident);
    }

    pub fn get(&self, id: Uuid) -> Option<Incident> {
        self.incidents.get(&id).map(|i| i.clone())
    }

    pub fn list_all(&self) -> Vec<Incident> {
        self.incidents.iter().map(|e| e.value().clone()).collect()
    }

    pub fn list_active(&self) -> Vec<Incident> {
        self.incidents
            .iter()
            .filter(|e| e.value().status == IncidentStatus::Active)
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn count(&self) -> usize {
        self.incidents.len()
    }
}

impl Default for IncidentRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::LogEntry;

    fn create_test_entry(severity: Severity) -> LogEntry {
        LogEntry::new_with_severity(
            Uuid::new_v4(),
            1,
            chrono::Utc::now(),
            "Test error".to_string(),
            severity,
        )
    }

    #[tokio::test]
    async fn test_incident_creation() {
        let detector = IncidentDetector::new();
        let entry = create_test_entry(Severity::Error);
        let signature = "test-sig".to_string();

        let incident = detector.create_incident(&signature, &entry, 5).await;

        assert_eq!(incident.status, IncidentStatus::Active);
        assert_eq!(incident.severity, Severity::Error);

        // Same pattern should update existing incident
        let incident2 = detector.create_incident(&signature, &entry, 6).await;
        assert_eq!(incident.id, incident2.id);
    }

    #[test]
    fn test_repository() {
        let repo = IncidentRepository::new();
        let incident = Incident::new("Test incident".to_string()).with_severity(Severity::Warn);

        repo.store(incident.clone());
        assert_eq!(repo.count(), 1);

        let retrieved = repo.get(incident.id).unwrap();
        assert_eq!(retrieved.title, "Test incident");
    }
}
