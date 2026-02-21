use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedEntry {
    pub mtime: f64,
    pub processed_at: DateTime<Utc>,
    pub result: String,
    pub text_chars: usize,
}

pub struct SyncState {
    conn: Connection,
}

impl SyncState {
    /// Open (or create) the SQLite database, run migrations, and auto-migrate
    /// from the old JSON state file if needed.
    pub fn open(config: &Config) -> Result<Self> {
        let db_path = config.db_path();
        let needs_migration = !db_path.exists();

        let state = Self::open_at(&db_path)?;

        if needs_migration {
            state.migrate_from_json(config);
        }

        Ok(state)
    }

    /// Open (or create) a SQLite database at an explicit path.
    ///
    /// This is the low-level constructor used by `open()` and by tests
    /// (which pass a tempfile path to avoid touching the production DB).
    pub fn open_at(db_path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open state database at {}", db_path.display()))?;

        // Restrict DB to owner-only (0600) since it contains session data
        let _ = std::fs::set_permissions(db_path, std::fs::Permissions::from_mode(0o600));

        // Enable WAL mode for crash safety and concurrent reads
        conn.execute_batch("PRAGMA journal_mode=WAL;").ok();

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS processed_sessions (
                session_id TEXT PRIMARY KEY,
                mtime REAL NOT NULL,
                processed_at TEXT NOT NULL,
                result TEXT NOT NULL,
                text_chars INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS session_stats (
                session_id TEXT PRIMARY KEY,
                total_duration_ms INTEGER,
                agent_time_ms INTEGER,
                user_time_ms INTEGER,
                real_user_message_count INTEGER,
                agent_turn_count INTEGER,
                write_count INTEGER,
                edit_count INTEGER,
                bash_count INTEGER,
                files_touched_count INTEGER,
                commit_ratio_pct REAL,
                computed_at TEXT
            );
            CREATE TABLE IF NOT EXISTS prompt_files (
                file_path TEXT,
                session_id TEXT,
                project_cwd TEXT,
                written_at TEXT,
                content_preview TEXT,
                prompt_confidence REAL,
                discovered_at TEXT,
                PRIMARY KEY (file_path, session_id)
            );",
        )
        .context("Failed to create state tables")?;

        Ok(Self { conn })
    }

    /// Check if a session has already been processed with the same mtime.
    pub fn is_processed(&self, session_id: &str, mtime: f64) -> bool {
        self.conn
            .query_row(
                "SELECT 1 FROM processed_sessions WHERE session_id = ? AND abs(mtime - ?) < 0.001",
                params![session_id, mtime],
                |_| Ok(()),
            )
            .is_ok()
    }

    /// Mark a session as processed (immediate write).
    pub fn mark_processed(
        &self,
        session_id: &str,
        mtime: f64,
        result: &str,
        text_chars: usize,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT OR REPLACE INTO processed_sessions (session_id, mtime, processed_at, result, text_chars)
                 VALUES (?, ?, ?, ?, ?)",
                params![session_id, mtime, now, result, text_chars as i64],
            )
            .context("Failed to write processed session")?;

        self.conn
            .execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES ('last_run', ?)",
                params![now],
            )
            .context("Failed to write last_run metadata")?;

        Ok(())
    }

    /// Get the last run timestamp.
    pub fn last_run(&self) -> Option<DateTime<Utc>> {
        self.conn
            .query_row(
                "SELECT value FROM metadata WHERE key = 'last_run'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|s| s.parse::<DateTime<Utc>>().ok())
    }

    /// Get all processed sessions (for status iteration).
    pub fn processed_sessions(&self) -> Result<HashMap<String, ProcessedEntry>> {
        let mut stmt = self
            .conn
            .prepare("SELECT session_id, mtime, processed_at, result, text_chars FROM processed_sessions")
            .context("Failed to prepare processed_sessions query")?;

        let rows = stmt
            .query_map([], |row| {
                let session_id: String = row.get(0)?;
                let mtime: f64 = row.get(1)?;
                let processed_at_str: String = row.get(2)?;
                let result: String = row.get(3)?;
                let text_chars: i64 = row.get(4)?;

                let processed_at = processed_at_str
                    .parse::<DateTime<Utc>>()
                    .unwrap_or_else(|_| Utc::now());

                Ok((
                    session_id,
                    ProcessedEntry {
                        mtime,
                        processed_at,
                        result,
                        text_chars: text_chars as usize,
                    },
                ))
            })
            .context("Failed to query processed sessions")?;

        let mut map = HashMap::new();
        for row in rows.flatten() {
            map.insert(row.0, row.1);
        }
        Ok(map)
    }

    /// Upsert session stats for a given session.
    pub fn upsert_session_stats(
        &self,
        session_id: &str,
        total_duration_ms: u64,
        agent_time_ms: u64,
        user_time_ms: u64,
        real_user_message_count: usize,
        agent_turn_count: usize,
        write_count: usize,
        edit_count: usize,
        bash_count: usize,
        files_touched_count: usize,
        commit_ratio_pct: Option<f64>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT OR REPLACE INTO session_stats
                 (session_id, total_duration_ms, agent_time_ms, user_time_ms,
                  real_user_message_count, agent_turn_count, write_count, edit_count,
                  bash_count, files_touched_count, commit_ratio_pct, computed_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    session_id,
                    total_duration_ms as i64,
                    agent_time_ms as i64,
                    user_time_ms as i64,
                    real_user_message_count as i64,
                    agent_turn_count as i64,
                    write_count as i64,
                    edit_count as i64,
                    bash_count as i64,
                    files_touched_count as i64,
                    commit_ratio_pct,
                    now,
                ],
            )
            .context("Failed to upsert session stats")?;
        Ok(())
    }

    /// Get stats for a specific session.
    pub fn get_session_stats(&self, session_id: &str) -> Option<SessionStatsRow> {
        self.conn
            .query_row(
                "SELECT session_id, total_duration_ms, agent_time_ms, user_time_ms,
                        real_user_message_count, agent_turn_count, write_count, edit_count,
                        bash_count, files_touched_count, commit_ratio_pct, computed_at
                 FROM session_stats WHERE session_id = ?",
                params![session_id],
                |row| SessionStatsRow::from_row(row),
            )
            .ok()
    }

    /// Get all session stats from the last N days.
    pub fn get_all_session_stats(&self, days: u32) -> Result<Vec<SessionStatsRow>> {
        let cutoff = (Utc::now() - chrono::Duration::days(days as i64)).to_rfc3339();
        let mut stmt = self
            .conn
            .prepare(
                "SELECT session_id, total_duration_ms, agent_time_ms, user_time_ms,
                        real_user_message_count, agent_turn_count, write_count, edit_count,
                        bash_count, files_touched_count, commit_ratio_pct, computed_at
                 FROM session_stats WHERE computed_at >= ?",
            )
            .context("Failed to prepare session_stats query")?;

        let rows = stmt
            .query_map(params![cutoff], |row| SessionStatsRow::from_row(row))
            .context("Failed to query session stats")?;

        Ok(rows.flatten().collect())
    }

    /// Upsert a discovered prompt file.
    pub fn upsert_prompt_file(
        &self,
        file_path: &str,
        session_id: &str,
        project_cwd: &str,
        written_at: &str,
        content_preview: &str,
        prompt_confidence: f64,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT OR REPLACE INTO prompt_files
                 (file_path, session_id, project_cwd, written_at, content_preview,
                  prompt_confidence, discovered_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                params![
                    file_path,
                    session_id,
                    project_cwd,
                    written_at,
                    content_preview,
                    prompt_confidence,
                    now,
                ],
            )
            .context("Failed to upsert prompt file")?;
        Ok(())
    }

    /// Get all prompt files, optionally filtered by days.
    pub fn get_prompt_files(&self, days: Option<u32>) -> Result<Vec<PromptFileRow>> {
        let query = match days {
            Some(d) => {
                let cutoff = (Utc::now() - chrono::Duration::days(d as i64)).to_rfc3339();
                let mut stmt = self
                    .conn
                    .prepare(
                        "SELECT file_path, session_id, project_cwd, written_at, content_preview,
                                prompt_confidence, discovered_at
                         FROM prompt_files WHERE discovered_at >= ?
                         ORDER BY prompt_confidence DESC",
                    )
                    .context("Failed to prepare prompt_files query")?;
                let rows = stmt
                    .query_map(params![cutoff], |row| PromptFileRow::from_row(row))
                    .context("Failed to query prompt files")?;
                return Ok(rows.flatten().collect());
            }
            None => {
                "SELECT file_path, session_id, project_cwd, written_at, content_preview,
                        prompt_confidence, discovered_at
                 FROM prompt_files ORDER BY prompt_confidence DESC"
            }
        };

        let mut stmt = self
            .conn
            .prepare(query)
            .context("Failed to prepare prompt_files query")?;
        let rows = stmt
            .query_map([], |row| PromptFileRow::from_row(row))
            .context("Failed to query prompt files")?;
        Ok(rows.flatten().collect())
    }

    /// Auto-migrate from old JSON state file if it exists.
    fn migrate_from_json(&self, config: &Config) {
        // Check old vault path location
        let old_path = config.vault_path().join(".sync-state.json");

        let json_path = if old_path.exists() {
            old_path
        } else {
            return;
        };

        log::info!(
            "Migrating state from {} to SQLite...",
            json_path.display()
        );

        let content = match std::fs::read_to_string(&json_path) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Could not read old state file: {}", e);
                return;
            }
        };

        // Parse the old JSON format
        let old_state: OldSyncState = match serde_json::from_str(&content) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Could not parse old state file: {}", e);
                return;
            }
        };

        // Insert all entries in a transaction
        let tx = match self.conn.unchecked_transaction() {
            Ok(t) => t,
            Err(e) => {
                log::warn!("Could not begin migration transaction: {}", e);
                return;
            }
        };

        let mut count = 0;
        for (session_id, entry) in &old_state.processed_sessions {
            let processed_at = entry.processed_at.to_rfc3339();
            if tx
                .execute(
                    "INSERT OR REPLACE INTO processed_sessions (session_id, mtime, processed_at, result, text_chars)
                     VALUES (?, ?, ?, ?, ?)",
                    params![session_id, entry.mtime, processed_at, entry.result, entry.text_chars as i64],
                )
                .is_ok()
            {
                count += 1;
            }
        }

        if let Some(last_run) = old_state.last_run {
            let _ = tx.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES ('last_run', ?)",
                params![last_run.to_rfc3339()],
            );
        }

        if let Err(e) = tx.commit() {
            log::warn!("Migration commit failed: {}", e);
            return;
        }

        log::info!("Migrated {} sessions from JSON to SQLite.", count);
    }
}

/// Old JSON state format for migration purposes.
#[derive(Deserialize)]
struct OldSyncState {
    processed_sessions: HashMap<String, ProcessedEntry>,
    last_run: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionStatsRow {
    pub session_id: String,
    pub total_duration_ms: u64,
    pub agent_time_ms: u64,
    pub user_time_ms: u64,
    pub real_user_message_count: usize,
    pub agent_turn_count: usize,
    pub write_count: usize,
    pub edit_count: usize,
    pub bash_count: usize,
    pub files_touched_count: usize,
    pub commit_ratio_pct: Option<f64>,
    pub computed_at: String,
}

impl SessionStatsRow {
    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            session_id: row.get(0)?,
            total_duration_ms: row.get::<_, i64>(1)? as u64,
            agent_time_ms: row.get::<_, i64>(2)? as u64,
            user_time_ms: row.get::<_, i64>(3)? as u64,
            real_user_message_count: row.get::<_, i64>(4)? as usize,
            agent_turn_count: row.get::<_, i64>(5)? as usize,
            write_count: row.get::<_, i64>(6)? as usize,
            edit_count: row.get::<_, i64>(7)? as usize,
            bash_count: row.get::<_, i64>(8)? as usize,
            files_touched_count: row.get::<_, i64>(9)? as usize,
            commit_ratio_pct: row.get(10)?,
            computed_at: row.get(11)?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PromptFileRow {
    pub file_path: String,
    pub session_id: String,
    pub project_cwd: String,
    pub written_at: String,
    pub content_preview: String,
    pub prompt_confidence: f64,
    pub discovered_at: String,
}

impl PromptFileRow {
    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            file_path: row.get(0)?,
            session_id: row.get(1)?,
            project_cwd: row.get(2)?,
            written_at: row.get(3)?,
            content_preview: row.get(4)?,
            prompt_confidence: row.get(5)?,
            discovered_at: row.get(6)?,
        })
    }
}
