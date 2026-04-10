//! Pattern tracking and detection (recurring errors, restart loops, new exceptions)

use crate::models::{LogEntry, Pattern, Severity};
use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::Instant;

/// Tracks patterns with sliding window frequency counting
pub struct PatternTracker {
    /// Active patterns being tracked
    patterns: DashMap<String, TrackedPattern>,
    /// Window duration for counting
    window_duration: Duration,
    /// Threshold for triggering "recurring error" detection
    recurring_threshold: u32,
    /// Known signatures (for "new exception" detection)
    known_signatures: Arc<RwLock<HashSet<String>>>,
    /// State machine for restart loop detection
    restart_detector: RestartLoopDetector,
}

/// Internal tracking state for a pattern
struct TrackedPattern {
    pattern: Pattern,
    window_start: Instant,
    window_count: u32,
}

/// Result of tracking a pattern
pub struct PatternState {
    pub window_count: u32,
    pub is_new: bool,
    pub is_recurring: bool,
    pub should_create_incident: bool,
}

impl PatternTracker {
    pub fn new() -> Self {
        Self {
            patterns: DashMap::new(),
            window_duration: Duration::from_secs(60), // 60s sliding window
            recurring_threshold: 5,
            known_signatures: Arc::new(RwLock::new(HashSet::new())),
            restart_detector: RestartLoopDetector::new(),
        }
    }

    /// Track a pattern occurrence
    pub async fn track(&self, signature: &str, entry: &LogEntry) -> PatternState {
        let now = Instant::now();

        // Check if this is a new pattern
        let is_new = !self.is_known(signature).await;
        if is_new {
            self.add_known_signature(signature).await;
        }

        // Update or create tracked pattern
        let mut window_count = 1u32;
        let mut is_recurring = false;

        match self.patterns.entry(signature.to_string()) {
            dashmap::mapref::entry::Entry::Occupied(mut occupied) => {
                let tracked = occupied.get_mut();

                // Check if window expired
                if now.duration_since(tracked.window_start) > self.window_duration {
                    // Reset window
                    tracked.window_start = now;
                    tracked.window_count = 1;
                } else {
                    // Increment count within current window
                    tracked.window_count += 1;
                    window_count = tracked.window_count;
                }

                // Update pattern metadata
                tracked.pattern.last_seen = entry.timestamp;
                tracked.pattern.occurrence_count += 1;
                if entry.severity > tracked.pattern.severity {
                    tracked.pattern.severity = entry.severity;
                }

                // Check recurring threshold
                if window_count >= self.recurring_threshold {
                    is_recurring = true;
                }
            }
            dashmap::mapref::entry::Entry::Vacant(vacant) => {
                // Create new pattern
                let pattern = Pattern::new(signature)
                    .with_severity(entry.severity)
                    .with_sample_entry(entry.id);

                vacant.insert(TrackedPattern {
                    pattern,
                    window_start: now,
                    window_count: 1,
                });
            }
        }

        // Check for restart loop
        self.restart_detector.check(entry);

        // Determine if we should create an incident
        let should_create_incident = is_recurring || (is_new && entry.severity >= Severity::Error);

        PatternState {
            window_count,
            is_new,
            is_recurring,
            should_create_incident,
        }
    }

    async fn is_known(&self, signature: &str) -> bool {
        let signatures = self.known_signatures.read().await;
        signatures.contains(signature)
    }

    async fn add_known_signature(&self, signature: &str) {
        let mut signatures = self.known_signatures.write().await;
        signatures.insert(signature.to_string());
    }

    /// Get a pattern by signature
    pub fn get_pattern(&self, signature: &str) -> Option<Pattern> {
        self.patterns.get(signature).map(|t| t.pattern.clone())
    }

    /// Get patterns exceeding threshold in current window
    pub fn get_recurring_patterns(&self, threshold: u32) -> Vec<(String, u32)> {
        let mut recurring = Vec::new();

        for entry in self.patterns.iter() {
            let tracked = entry.value();
            if tracked.window_count >= threshold {
                recurring.push((entry.key().clone(), tracked.window_count));
            }
        }

        recurring
    }

    /// Get count of active patterns
    pub fn active_pattern_count(&self) -> usize {
        self.patterns.len()
    }

    /// Check for restart loop in a specific service
    pub fn check_restart_loop(&self, service: &str) -> bool {
        self.restart_detector.is_in_loop(service)
    }
}

impl Default for PatternTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Tracks state transitions for restart loop detection
#[derive(Clone, Debug)]
struct ServiceTransition {
    state: ServiceState,
    transition_count: u32,
    first_transition: Instant,
}

/// State machine for detecting restart loops
pub struct RestartLoopDetector {
    /// Service state tracking with transition history
    service_states: DashMap<String, ServiceTransition>,
    /// Window for restart loop detection
    loop_window: Duration,
}

#[derive(Clone, Debug)]
enum ServiceState {
    Unknown,
    Starting { since: Instant },
    Running { since: Instant },
    Stopping { since: Instant },
    Stopped { since: Instant },
}

impl RestartLoopDetector {
    pub fn new() -> Self {
        Self {
            service_states: DashMap::new(),
            loop_window: Duration::from_secs(30), // 30s window
        }
    }

    /// Check a log entry for restart patterns
    pub fn check(&self, entry: &LogEntry) {
        let content = entry.raw_content.to_lowercase();

        // Extract service name from entry or use "unknown"
        let service = entry.service.as_deref().unwrap_or("unknown").to_string();

        // Detect state transitions
        let new_state = if content.contains("starting") || content.contains("start ") {
            Some(ServiceState::Starting {
                since: Instant::now(),
            })
        } else if content.contains("stopping") || content.contains("stop ") {
            Some(ServiceState::Stopping {
                since: Instant::now(),
            })
        } else if content.contains("stopped") || content.contains("shutdown") {
            Some(ServiceState::Stopped {
                since: Instant::now(),
            })
        } else if content.contains("started") || content.contains("ready") {
            Some(ServiceState::Running {
                since: Instant::now(),
            })
        } else {
            None
        };

        if let Some(state) = new_state {
            let now = Instant::now();
            match self.service_states.entry(service) {
                dashmap::mapref::entry::Entry::Occupied(mut occupied) => {
                    let trans = occupied.get_mut();
                    // Check if window expired, reset if so
                    if now.duration_since(trans.first_transition) > self.loop_window {
                        trans.transition_count = 1;
                        trans.first_transition = now;
                    } else {
                        trans.transition_count += 1;
                    }
                    trans.state = state;
                }
                dashmap::mapref::entry::Entry::Vacant(vacant) => {
                    vacant.insert(ServiceTransition {
                        state,
                        transition_count: 1,
                        first_transition: now,
                    });
                }
            }
        }
    }

    /// Check if a service is currently in a restart loop
    /// A loop requires at least 2 transitions within the window (e.g., start->stop)
    pub fn is_in_loop(&self, service: &str) -> bool {
        if let Some(trans) = self.service_states.get(service) {
            // Need at least 2 transitions to be a loop, and within window
            trans.transition_count >= 2 && trans.first_transition.elapsed() < self.loop_window
        } else {
            false
        }
    }

    /// Get services currently in restart loops
    pub fn get_looping_services(&self) -> Vec<String> {
        self.service_states
            .iter()
            .filter(|entry| self.is_in_loop(entry.key()))
            .map(|entry| entry.key().clone())
            .collect()
    }
}

impl Default for RestartLoopDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// New exception detector for first-seen errors
pub struct NewExceptionDetector {
    seen_signatures: Arc<RwLock<HashSet<String>>>,
}

impl NewExceptionDetector {
    pub fn new() -> Self {
        Self {
            seen_signatures: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Check if this is a new (never seen) exception
    pub async fn is_new(&self, signature: &str) -> bool {
        let signatures = self.seen_signatures.read().await;
        !signatures.contains(signature)
    }

    /// Mark a signature as seen
    pub async fn mark_seen(&self, signature: &str) {
        let mut signatures = self.seen_signatures.write().await;
        signatures.insert(signature.to_string());
    }
}

impl Default for NewExceptionDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entry(content: &str, severity: Severity) -> LogEntry {
        LogEntry::new_with_severity(
            uuid::Uuid::new_v4(),
            1,
            chrono::Utc::now(),
            content.to_string(),
            severity,
        )
    }

    #[tokio::test]
    async fn test_recurring_error_detection() {
        let tracker = PatternTracker::new();
        let signature = "test-error-sig".to_string();

        // Add 5 occurrences within window
        for i in 0..5 {
            let entry = create_test_entry("ERROR: Database connection failed", Severity::Error);
            let state = tracker.track(&signature, &entry).await;

            if i < 4 {
                assert!(!state.is_recurring);
            } else {
                assert!(state.is_recurring);
                assert!(state.should_create_incident);
            }
        }
    }

    #[test]
    fn test_restart_loop_detection() {
        let detector = RestartLoopDetector::new();

        // Simulate start -> stop -> start sequence
        let mut start_entry = create_test_entry("INFO: Starting checkout-service", Severity::Info);
        start_entry.service = Some("checkout-service".to_string());

        detector.check(&start_entry);
        assert!(!detector.is_in_loop("checkout-service"));

        let mut stop_entry =
            create_test_entry("INFO: Stopping checkout-service (SIGTERM)", Severity::Info);
        stop_entry.service = Some("checkout-service".to_string());

        detector.check(&stop_entry);
        // Should detect potential loop if another start comes quickly
    }
}
