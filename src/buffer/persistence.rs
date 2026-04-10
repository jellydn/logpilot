//! Persistence layer for high-severity log entries
//!
//! Stores ERROR and FATAL entries in SQLite for historical analysis

use crate::error::{LogPilotError, Result};
use crate::models::{LogEntry, Severity};
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Row, Sqlite};
use std::collections::HashMap;
use uuid::Uuid;

/// SQLite persistence for high-severity logs
pub struct PersistenceStore {
    pool: Pool<Sqlite>,
}

impl PersistenceStore {
    /// Create a new persistence store
    pub async fn new(db_path: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&format!("sqlite:{}", db_path))
            .await?;

        // Initialize schema
        Self::init_schema(&pool).await?;

        Ok(Self { pool })
    }

    /// Create an in-memory persistence store for testing
    pub async fn new_in_memory() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;

        Self::init_schema(&pool).await?;

        Ok(Self { pool })
    }

    async fn init_schema(pool: &Pool<Sqlite>) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS log_entries (
                id TEXT PRIMARY KEY,
                pane_id TEXT NOT NULL,
                sequence INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                severity TEXT NOT NULL,
                service TEXT,
                raw_content TEXT NOT NULL,
                parsed_fields TEXT,
                received_at TEXT NOT NULL
            );
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_entries_timestamp ON log_entries(timestamp);
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_entries_severity ON log_entries(severity);
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_entries_pane ON log_entries(pane_id);
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Store a log entry if it meets severity threshold
    pub async fn store_entry(&self, entry: &LogEntry, min_severity: Severity) -> Result<bool> {
        if entry.severity < min_severity {
            return Ok(false);
        }

        let parsed_fields_json = serde_json::to_string(&entry.parsed_fields)
            .map_err(|e| LogPilotError::db_op(format!("JSON serialize error: {}", e)))?;

        sqlx::query(
            r#"
            INSERT INTO log_entries 
            (id, pane_id, sequence, timestamp, severity, service, raw_content, parsed_fields, received_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
        )
        .bind(entry.id.to_string())
        .bind(entry.pane_id.to_string())
        .bind(entry.sequence as i64)
        .bind(entry.timestamp.to_rfc3339())
        .bind(format!("{:?}", entry.severity))
        .bind(entry.service.as_ref())
        .bind(&entry.raw_content)
        .bind(&parsed_fields_json)
        .bind(entry.received_at.to_rfc3339())
        .execute(&self.pool)
        .await
        ?;

        Ok(true)
    }

    /// Query entries by time range
    pub async fn query_entries(
        &self,
        since: DateTime<Utc>,
        until: DateTime<Utc>,
        severity: Option<Severity>,
    ) -> Result<Vec<LogEntry>> {
        let mut query =
            String::from("SELECT * FROM log_entries WHERE timestamp >= ?1 AND timestamp <= ?2");

        if severity.is_some() {
            query.push_str(" AND severity = ?3");
        }

        query.push_str(" ORDER BY timestamp DESC");

        let mut sql_query = sqlx::query_as::<_, LogEntryRow>(&query)
            .bind(since.to_rfc3339())
            .bind(until.to_rfc3339());

        if let Some(sev) = severity {
            sql_query = sql_query.bind(format!("{:?}", sev));
        }

        let rows = sql_query.fetch_all(&self.pool).await?;

        rows.into_iter().map(|r| r.to_entry()).collect()
    }

    /// Get entries by pane
    pub async fn entries_for_pane(&self, pane_id: Uuid) -> Result<Vec<LogEntry>> {
        let rows = sqlx::query_as::<_, LogEntryRow>(
            "SELECT * FROM log_entries WHERE pane_id = ?1 ORDER BY timestamp DESC",
        )
        .bind(pane_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.to_entry()).collect()
    }

    /// Get entry count by severity
    pub async fn count_by_severity(&self, since: DateTime<Utc>) -> Result<HashMap<Severity, i64>> {
        let rows = sqlx::query(
            "SELECT severity, COUNT(*) as count FROM log_entries WHERE timestamp >= ?1 GROUP BY severity",
        )
        .bind(since.to_rfc3339())
        .fetch_all(&self.pool)
        .await
        ?;

        let mut counts = HashMap::new();
        for row in rows {
            let sev_str: String = row.try_get("severity").unwrap_or_default();
            let count: i64 = row.try_get("count").unwrap_or(0);
            let severity = Self::parse_severity(&sev_str);
            counts.insert(severity, count);
        }

        Ok(counts)
    }

    /// Cleanup old entries
    pub async fn cleanup_before(&self, cutoff: DateTime<Utc>) -> Result<u64> {
        let result = sqlx::query("DELETE FROM log_entries WHERE timestamp < ?1")
            .bind(cutoff.to_rfc3339())
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    fn parse_severity(s: &str) -> Severity {
        match s {
            "Trace" => Severity::Trace,
            "Debug" => Severity::Debug,
            "Info" => Severity::Info,
            "Warn" => Severity::Warn,
            "Error" => Severity::Error,
            "Fatal" => Severity::Fatal,
            _ => Severity::Unknown,
        }
    }
}

/// Database row representation for LogEntry
#[derive(sqlx::FromRow)]
struct LogEntryRow {
    id: String,
    pane_id: String,
    sequence: i64,
    timestamp: String,
    severity: String,
    service: Option<String>,
    raw_content: String,
    parsed_fields: String,
    received_at: String,
}

#[allow(clippy::wrong_self_convention)]
impl LogEntryRow {
    fn to_entry(self) -> Result<LogEntry> {
        let parsed_fields: HashMap<String, String> = serde_json::from_str(&self.parsed_fields)
            .map_err(|e| LogPilotError::db_op(format!("JSON parse error: {}", e)))?;

        Ok(LogEntry {
            id: Uuid::parse_str(&self.id)
                .map_err(|e| LogPilotError::db_op(format!("UUID parse error: {}", e)))?,
            pane_id: Uuid::parse_str(&self.pane_id)
                .map_err(|e| LogPilotError::db_op(format!("UUID parse error: {}", e)))?,
            sequence: self.sequence as u64,
            timestamp: DateTime::parse_from_rfc3339(&self.timestamp)
                .map_err(|e| LogPilotError::db_op(format!("Date parse error: {}", e)))?
                .with_timezone(&Utc),
            severity: Self::parse_severity(&self.severity),
            service: self.service,
            raw_content: self.raw_content,
            parsed_fields,
            received_at: DateTime::parse_from_rfc3339(&self.received_at)
                .map_err(|e| LogPilotError::db_op(format!("Date parse error: {}", e)))?
                .with_timezone(&Utc),
        })
    }

    fn parse_severity(s: &str) -> Severity {
        match s {
            "Trace" => Severity::Trace,
            "Debug" => Severity::Debug,
            "Info" => Severity::Info,
            "Warn" => Severity::Warn,
            "Error" => Severity::Error,
            "Fatal" => Severity::Fatal,
            _ => Severity::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_entry(content: &str, severity: Severity) -> LogEntry {
        LogEntry {
            id: Uuid::new_v4(),
            pane_id: Uuid::new_v4(),
            sequence: 1,
            timestamp: Utc::now(),
            severity,
            service: Some("test-service".to_string()),
            raw_content: content.to_string(),
            parsed_fields: HashMap::new(),
            received_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_store_and_query() {
        let store = PersistenceStore::new_in_memory().await.unwrap();

        let entry = create_test_entry("Error occurred", Severity::Error);
        let stored = store.store_entry(&entry, Severity::Error).await.unwrap();
        assert!(stored);

        // Info entry should not be stored
        let info_entry = create_test_entry("Info message", Severity::Info);
        let stored = store
            .store_entry(&info_entry, Severity::Error)
            .await
            .unwrap();
        assert!(!stored);

        // Query entries
        let entries = store
            .query_entries(
                Utc::now() - chrono::Duration::minutes(1),
                Utc::now() + chrono::Duration::minutes(1),
                None,
            )
            .await
            .unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_content, "Error occurred");
    }

    #[tokio::test]
    async fn test_count_by_severity() {
        let store = PersistenceStore::new_in_memory().await.unwrap();

        // Store multiple entries
        for _ in 0..5 {
            store
                .store_entry(&create_test_entry("Error", Severity::Error), Severity::Info)
                .await
                .unwrap();
        }
        for _ in 0..3 {
            store
                .store_entry(&create_test_entry("Fatal", Severity::Fatal), Severity::Info)
                .await
                .unwrap();
        }

        let counts = store
            .count_by_severity(Utc::now() - chrono::Duration::minutes(1))
            .await
            .unwrap();
        assert_eq!(counts.get(&Severity::Error), Some(&5));
        assert_eq!(counts.get(&Severity::Fatal), Some(&3));
    }

    #[tokio::test]
    async fn test_cleanup() {
        let store = PersistenceStore::new_in_memory().await.unwrap();

        // Store old entry
        let mut old_entry = create_test_entry("Old error", Severity::Error);
        old_entry.timestamp = Utc::now() - chrono::Duration::days(7);
        store.store_entry(&old_entry, Severity::Info).await.unwrap();

        // Store new entry
        store
            .store_entry(
                &create_test_entry("New error", Severity::Error),
                Severity::Info,
            )
            .await
            .unwrap();

        // Cleanup entries older than 1 day
        let deleted = store
            .cleanup_before(Utc::now() - chrono::Duration::days(1))
            .await
            .unwrap();
        assert_eq!(deleted, 1);

        // Verify only new entry remains
        let entries = store
            .query_entries(Utc::now() - chrono::Duration::days(7), Utc::now(), None)
            .await
            .unwrap();
        assert_eq!(entries.len(), 1);
    }
}
