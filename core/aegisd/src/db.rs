//! SQLite 기반 영속 저장소

use rusqlite::{Connection, Result as SqlResult};
use std::path::Path;

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
        self.conn.execute_batch("
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
        ")?;
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
}
