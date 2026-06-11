//! SQLite 기반 영속 저장소

use aegis_common::types::{AegisEvent, EventSeverity, EventType, QuarantineEntry};
use rusqlite::{Connection, Result as SqlResult};
use std::path::Path;
use uuid::Uuid;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub async fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let conn = Connection::open(path.as_ref())?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> SqlResult<()> {
        self.conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS jobs (
                id          TEXT PRIMARY KEY,
                job_type    TEXT NOT NULL,
                status      TEXT NOT NULL,
                created_at  TEXT NOT NULL,
                started_at  TEXT,
                completed_at TEXT,
                error       TEXT,
                data        TEXT
            );

            CREATE TABLE IF NOT EXISTS events (
                id          TEXT PRIMARY KEY,
                timestamp   TEXT NOT NULL,
                event_type  TEXT NOT NULL,
                severity    TEXT NOT NULL,
                message     TEXT NOT NULL,
                context     TEXT
            );

            CREATE TABLE IF NOT EXISTS quarantine (
                id              TEXT PRIMARY KEY,
                original_path   TEXT NOT NULL,
                quarantine_path TEXT NOT NULL,
                threat_json     TEXT NOT NULL,
                quarantined_at  TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_events_ts ON events(timestamp);
            CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status);
        ",
        )?;
        Ok(())
    }

    pub fn log_event(
        &self,
        id: &str,
        timestamp: &str,
        event_type: &str,
        severity: &str,
        message: &str,
        context: Option<&str>,
    ) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO events
             (id, timestamp, event_type, severity, message, context)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, timestamp, event_type, severity, message, context],
        )?;
        Ok(())
    }

    pub fn log_aegis_event(&self, event: &AegisEvent) -> SqlResult<()> {
        let event_type = serde_json::to_string(&event.event_type).unwrap_or_else(|_| "UserAction".into());
        let severity = serde_json::to_string(&event.severity).unwrap_or_else(|_| "Info".into());
        let context = event.context.to_string();
        self.log_event(
            &event.id.to_string(),
            &event.timestamp.to_rfc3339(),
            &event_type,
            &severity,
            &event.message,
            Some(&context),
        )
    }

    pub fn list_events(
        &self,
        since: Option<&str>,
        limit: u32,
    ) -> anyhow::Result<Vec<AegisEvent>> {
        let limit = limit.max(1).min(1000) as i64;
        let mut events = Vec::new();

        if let Some(since) = since {
            let mut stmt = self.conn.prepare(
                "SELECT id, timestamp, event_type, severity, message, context
                 FROM events WHERE timestamp >= ?1 ORDER BY timestamp DESC LIMIT ?2",
            )?;
            let rows = stmt.query_map(rusqlite::params![since, limit], row_to_event)?;
            for row in rows {
                events.push(row?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, timestamp, event_type, severity, message, context
                 FROM events ORDER BY timestamp DESC LIMIT ?1",
            )?;
            let rows = stmt.query_map(rusqlite::params![limit], row_to_event)?;
            for row in rows {
                events.push(row?);
            }
        }
        Ok(events)
    }

    pub fn insert_quarantine(&self, entry: &QuarantineEntry) -> SqlResult<()> {
        let threat_json = serde_json::to_string(&entry.threat).unwrap_or_else(|_| "{}".into());
        self.conn.execute(
            "INSERT OR REPLACE INTO quarantine
             (id, original_path, quarantine_path, threat_json, quarantined_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                entry.id.to_string(),
                entry.original_path.to_string_lossy(),
                entry.quarantine_path.to_string_lossy(),
                threat_json,
                entry.quarantined_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn delete_quarantine(&self, id: &str) -> SqlResult<usize> {
        self.conn
            .execute("DELETE FROM quarantine WHERE id = ?1", rusqlite::params![id])
    }

    pub fn load_quarantine(&self) -> anyhow::Result<Vec<QuarantineEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, original_path, quarantine_path, threat_json, quarantined_at
             FROM quarantine ORDER BY quarantined_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let original_path: String = row.get(1)?;
            let quarantine_path: String = row.get(2)?;
            let threat_json: String = row.get(3)?;
            let quarantined_at: String = row.get(4)?;
            let threat = serde_json::from_str(&threat_json).unwrap_or_else(|_| {
                aegis_common::types::ThreatInfo {
                    id: Uuid::nil(),
                    path: original_path.clone().into(),
                    verdict: aegis_common::types::ThreatVerdict::Suspicious,
                    risk_score: 0.0,
                    malware_family: None,
                    malware_type: None,
                    detections: vec![],
                    hash_sha256: String::new(),
                    size_bytes: 0,
                    detected_at: chrono::Utc::now(),
                }
            });
            Ok(QuarantineEntry {
                id: Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::new_v4()),
                original_path: original_path.into(),
                quarantine_path: quarantine_path.into(),
                threat,
                quarantined_at: chrono::DateTime::parse_from_rfc3339(&quarantined_at)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                can_restore: true,
            })
        })?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }
        Ok(entries)
    }
}

fn row_to_event(row: &rusqlite::Row<'_>) -> SqlResult<AegisEvent> {
    let id: String = row.get(0)?;
    let timestamp: String = row.get(1)?;
    let event_type_raw: String = row.get(2)?;
    let severity_raw: String = row.get(3)?;
    let message: String = row.get(4)?;
    let context_raw: Option<String> = row.get(5)?;

    let event_type: EventType = serde_json::from_str(&event_type_raw)
        .unwrap_or(EventType::UserAction);
    let severity: EventSeverity = serde_json::from_str(&severity_raw)
        .unwrap_or(EventSeverity::Info);
    let context = context_raw
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or(serde_json::Value::Null);

    Ok(AegisEvent {
        id: Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::new_v4()),
        timestamp: chrono::DateTime::parse_from_rfc3339(&timestamp)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        event_type,
        severity,
        message,
        context,
    })
}
