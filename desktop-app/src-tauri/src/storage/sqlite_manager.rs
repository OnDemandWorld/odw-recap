use crate::error::{RecapError, Result};
use crate::storage::types::*;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct SqliteManager {
    conn: Connection,
}

/// Parse a UUID stored in the database, propagating an error instead of
/// silently inventing a new ID (which would corrupt lookups).
fn parse_stored_uuid(raw: &str) -> Result<Uuid> {
    Uuid::parse_str(raw).map_err(|e| {
        RecapError::Database(format!("Invalid UUID '{}' in database: {}", raw, e))
    })
}

/// Parse an enum column losslessly: unknown values surface as errors instead
/// of silently mapping to a default variant.
fn parse_enum<T: TryFrom<String, Error = String>>(row: &rusqlite::Row, name: &str) -> rusqlite::Result<T> {
    let raw: String = row.get(name)?;
    T::try_from(raw.clone()).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("{} (column '{}')", e, name),
            )),
        )
    })
}

impl SqliteManager {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let manager = Self { conn };
        manager.migrate()?;
        Ok(manager)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(MIGRATIONS)?;
        Ok(())
    }

    pub fn create_meeting(&self, meeting: &Meeting) -> Result<()> {
        self.conn.execute(
            "INSERT INTO meetings (
                id, title, started_at, ended_at, duration_seconds, status,
                audio_file_path, audio_format, audio_size_bytes, model_used, tags,
                folder_id, created_at, updated_at, sync_status, meeting_type,
                location, participants, language, topic, audio_source, stt_provider,
                llm_provider, prompt_template_used
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)",
            params![
                meeting.id.to_string(),
                meeting.title,
                meeting.started_at,
                meeting.ended_at,
                meeting.duration_seconds,
                meeting.status.to_string(),
                meeting.audio_file_path.to_string_lossy().to_string(),
                meeting.audio_format,
                meeting.audio_size_bytes,
                meeting.model_used,
                serde_json::to_string(&meeting.tags)?,
                meeting.folder_id.map(|id| id.to_string()),
                meeting.created_at,
                meeting.updated_at,
                meeting.sync_status.to_string(),
                meeting.meeting_type,
                meeting.location,
                serde_json::to_string(&meeting.participants)?,
                meeting.language,
                meeting.topic,
                meeting.audio_source.to_string(),
                meeting.stt_provider,
                meeting.llm_provider,
                meeting.prompt_template_used,
            ],
        )?;
        Ok(())
    }

    pub fn get_meeting(&self, id: Uuid) -> Result<Option<Meeting>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM meetings WHERE id = ?1"
        )?;

        let meeting = stmt.query_row([id.to_string()], |row| {
            Self::row_to_meeting(row)
        }).optional()?;

        Ok(meeting)
    }

    pub fn update_meeting(&self, meeting: &Meeting) -> Result<()> {
        self.conn.execute(
            "UPDATE meetings SET
                title = ?2, started_at = ?3, ended_at = ?4, duration_seconds = ?5,
                status = ?6, audio_file_path = ?7, audio_format = ?8,
                audio_size_bytes = ?9, model_used = ?10, tags = ?11,
                folder_id = ?12, updated_at = ?13, sync_status = ?14,
                meeting_type = ?15, location = ?16, participants = ?17,
                language = ?18, topic = ?19, audio_source = ?20,
                stt_provider = ?21, llm_provider = ?22, prompt_template_used = ?23
            WHERE id = ?1",
            params![
                meeting.id.to_string(),
                meeting.title,
                meeting.started_at,
                meeting.ended_at,
                meeting.duration_seconds,
                meeting.status.to_string(),
                meeting.audio_file_path.to_string_lossy().to_string(),
                meeting.audio_format,
                meeting.audio_size_bytes,
                meeting.model_used,
                serde_json::to_string(&meeting.tags)?,
                meeting.folder_id.map(|id| id.to_string()),
                meeting.updated_at,
                meeting.sync_status.to_string(),
                meeting.meeting_type,
                meeting.location,
                serde_json::to_string(&meeting.participants)?,
                meeting.language,
                meeting.topic,
                meeting.audio_source.to_string(),
                meeting.stt_provider,
                meeting.llm_provider,
                meeting.prompt_template_used,
            ],
        )?;
        Ok(())
    }

    pub fn list_meetings(&self, limit: i64, offset: i64) -> Result<Vec<Meeting>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM meetings ORDER BY started_at DESC LIMIT ?1 OFFSET ?2"
        )?;

        let meetings = stmt.query_map([limit, offset], |row| {
            Self::row_to_meeting(row)
        })?;

        let mut result = Vec::new();
        for meeting in meetings {
            result.push(meeting?);
        }
        Ok(result)
    }

    fn row_to_meeting(row: &rusqlite::Row) -> rusqlite::Result<Meeting> {
        // Map conversion errors into rusqlite errors so a corrupt row surfaces
        // instead of being silently replaced with a random ID.
        let parse_uuid = |name: &str| -> rusqlite::Result<Uuid> {
            let raw: String = row.get(name)?;
            Uuid::parse_str(&raw).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("invalid UUID '{}' in column '{}': {}", raw, name, e),
                    )),
                )
            })
        };

        Ok(Meeting {
            id: parse_uuid("id")?,
            title: row.get("title")?,
            started_at: row.get("started_at")?,
            ended_at: row.get("ended_at")?,
            duration_seconds: row.get("duration_seconds")?,
            status: parse_enum::<MeetingStatus>(row, "status")?,
            audio_file_path: PathBuf::from(row.get::<_, String>("audio_file_path")?),
            audio_format: row.get("audio_format")?,
            audio_size_bytes: row.get("audio_size_bytes")?,
            model_used: row.get("model_used")?,
            tags: serde_json::from_str(&row.get::<_, String>("tags")?).unwrap_or_default(),
            folder_id: match row.get::<_, Option<String>>("folder_id")? {
                Some(s) => Some(Uuid::parse_str(&s).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("invalid folder_id UUID '{}': {}", s, e),
                        )),
                    )
                })?),
                None => None,
            },
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            sync_status: parse_enum::<SyncStatus>(row, "sync_status")?,
            meeting_type: row.get("meeting_type")?,
            location: row.get("location")?,
            participants: serde_json::from_str(&row.get::<_, String>("participants")?)
                .unwrap_or_default(),
            language: row.get("language")?,
            topic: row.get("topic")?,
            audio_source: parse_enum::<AudioSource>(row, "audio_source")?,
            stt_provider: row.get("stt_provider")?,
            llm_provider: row.get("llm_provider")?,
            prompt_template_used: row.get("prompt_template_used")?,
        })
    }

    pub fn search_transcripts(&self, query: &str, limit: i64) -> Result<Vec<(Uuid, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, tseg.text FROM transcript_search ts
             JOIN transcript_segments tseg ON ts.rowid = tseg.id
             JOIN meetings m ON tseg.meeting_id = m.id
             WHERE transcript_search MATCH ?1
             LIMIT ?2"
        )?;

        let results = stmt.query_map(params![query, limit], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut output = Vec::new();
        for result in results {
            let (id_raw, text) = result?;
            output.push((parse_stored_uuid(&id_raw)?, text));
        }
        Ok(output)
    }

    /// Insert a transcript segment. Triggers keep the `transcript_search`
    /// FTS5 index in sync (see migrations).
    pub fn insert_transcript_segment(&self, segment: &TranscriptSegment) -> Result<()> {
        self.conn.execute(
            "INSERT INTO transcript_segments (
                meeting_id, start_ms, end_ms, text, speaker_id, confidence, is_final, version
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                segment.meeting_id.to_string(),
                segment.start_ms,
                segment.end_ms,
                segment.text,
                segment.speaker_id,
                segment.confidence,
                if segment.is_final { 1 } else { 0 },
                segment.version,
            ],
        )?;
        Ok(())
    }

    /// Load all transcript segments for a meeting, ordered by start time.
    pub fn get_transcript_segments(&self, meeting_id: Uuid) -> Result<Vec<TranscriptSegment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, start_ms, end_ms, text, speaker_id, confidence, is_final, version
             FROM transcript_segments WHERE meeting_id = ?1 ORDER BY start_ms ASC",
        )?;

        let rows = stmt.query_map([meeting_id.to_string()], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, f64>(6)?,
                row.get::<_, i32>(7)?,
                row.get::<_, i32>(8)?,
            ))
        })?;

        let mut segments = Vec::new();
        for row in rows {
            let (id, meeting_raw, start_ms, end_ms, text, speaker_id, confidence, is_final, version) = row?;
            segments.push(TranscriptSegment {
                id,
                meeting_id: parse_stored_uuid(&meeting_raw)?,
                start_ms,
                end_ms,
                text,
                speaker_id,
                confidence: confidence as f32,
                is_final: is_final != 0,
                version,
            });
        }
        Ok(segments)
    }

    /// Save (or replace) the summary for a meeting. Keeps the most recent
    /// summary per meeting by deleting any previous one first.
    pub fn save_summary(&self, summary: &Summary) -> Result<()> {
        self.conn.execute(
            "DELETE FROM summaries WHERE meeting_id = ?1",
            params![summary.meeting_id.to_string()],
        )?;
        self.conn.execute(
            "INSERT INTO summaries (
                meeting_id, content, generation_mode, model_used, created_at, updated_at, user_edited
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                summary.meeting_id.to_string(),
                summary.content,
                summary.generation_mode,
                summary.model_used,
                summary.created_at,
                summary.updated_at,
                if summary.user_edited { 1 } else { 0 },
            ],
        )?;
        Ok(())
    }

    /// Get the current summary for a meeting, if any.
    pub fn get_summary(&self, meeting_id: Uuid) -> Result<Option<Summary>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, content, generation_mode, model_used, created_at, updated_at, user_edited
             FROM summaries WHERE meeting_id = ?1 ORDER BY id DESC LIMIT 1",
        )?;

        let result = stmt
            .query_row(params![meeting_id.to_string()], |row| {
                let meeting_raw: String = row.get(1)?;
                let user_edited: i32 = row.get(7)?;
                Ok((
                    row.get::<_, i64>(0)?,
                    meeting_raw,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    user_edited,
                ))
            })
            .optional()?;

        match result {
            Some((id, meeting_raw, content, generation_mode, model_used, created_at, updated_at, user_edited)) => {
                Ok(Some(Summary {
                    id,
                    meeting_id: parse_stored_uuid(&meeting_raw)?,
                    content,
                    generation_mode,
                    model_used,
                    created_at,
                    updated_at,
                    user_edited: user_edited != 0,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn save_api_key(&self, provider: &str, key_encrypted: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO api_keys (provider, key_encrypted, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![provider, key_encrypted, chrono::Utc::now().timestamp_millis(), chrono::Utc::now().timestamp_millis()],
        )?;
        Ok(())
    }

    pub fn get_api_key(&self, provider: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT key_encrypted FROM api_keys WHERE provider = ?1")?;
        let key: Option<String> = stmt.query_row([provider], |row| {
            row.get("key_encrypted")
        }).optional()?;
        Ok(key)
    }

    /// List all stored API keys in encrypted form (provider, ciphertext).
    /// Used by the passphrase-change flow to re-encrypt every key.
    pub fn list_api_keys_encrypted(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare("SELECT provider, key_encrypted FROM api_keys")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut keys = Vec::new();
        for row in rows {
            keys.push(row?);
        }
        Ok(keys)
    }

    /// Atomically apply the effects of a passphrase change: replace every API
    /// key's ciphertext and rewrite the vault sentinel. A single transaction
    /// guarantees either everything moves to the new key or nothing does.
    pub fn apply_reencryption(&self, updates: &[(String, String)], vault_check_value: &str) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for (provider, key_encrypted) in updates {
            tx.execute(
                "UPDATE api_keys SET key_encrypted = ?2, updated_at = ?3 WHERE provider = ?1",
                params![provider, key_encrypted, chrono::Utc::now().timestamp_millis()],
            )?;
        }
        tx.execute(
            "INSERT OR REPLACE INTO configuration (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params!["vault_check", vault_check_value, chrono::Utc::now().timestamp_millis()],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn save_prompt_template(&self, name: &str, content: &str, meeting_type: Option<&str>, is_custom: bool) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO prompt_templates (name, content, meeting_type, is_custom, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![name, content, meeting_type, if is_custom { 1 } else { 0 }, chrono::Utc::now().timestamp_millis()],
        )?;
        Ok(())
    }

    pub fn get_prompt_template(&self, name: &str) -> Result<Option<(String, Option<String>, bool)>> {
        let mut stmt = self.conn.prepare("SELECT content, meeting_type, is_custom FROM prompt_templates WHERE name = ?1")?;
        let result = stmt.query_row([name], |row| {
            Ok((
                row.get::<_, String>("content")?,
                row.get::<_, Option<String>>("meeting_type")?,
                row.get::<_, i32>("is_custom")? == 1,
            ))
        }).optional()?;
        Ok(result)
    }

    /// List all prompt templates ordered by name. Returns
    /// `(name, content, meeting_type, is_custom)` tuples.
    pub fn list_prompt_templates(&self) -> Result<Vec<(String, String, Option<String>, bool)>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, content, meeting_type, is_custom FROM prompt_templates ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i32>(3)? == 1,
            ))
        })?;

        let mut templates = Vec::new();
        for row in rows {
            templates.push(row?);
        }
        Ok(templates)
    }

    pub fn get_config_value(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT value FROM configuration WHERE key = ?1")?;
        let value: Option<String> = stmt.query_row([key], |row| row.get(0)).optional()?;
        Ok(value)
    }

    pub fn set_config_value(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO configuration (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, value, chrono::Utc::now().timestamp_millis()],
        )?;
        Ok(())
    }
}

const MIGRATIONS: &str = r#"
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS meetings (
    id TEXT PRIMARY KEY,
    title TEXT,
    started_at INTEGER NOT NULL,
    ended_at INTEGER,
    duration_seconds INTEGER,
    status TEXT NOT NULL DEFAULT 'recording',
    audio_file_path TEXT NOT NULL,
    audio_format TEXT NOT NULL,
    audio_size_bytes INTEGER NOT NULL DEFAULT 0,
    model_used TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]',
    folder_id TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    sync_status TEXT NOT NULL DEFAULT 'not_synced',
    meeting_type TEXT,
    location TEXT,
    participants TEXT NOT NULL DEFAULT '[]',
    language TEXT NOT NULL DEFAULT 'en',
    topic TEXT,
    audio_source TEXT NOT NULL DEFAULT 'system_capture',
    stt_provider TEXT NOT NULL DEFAULT 'whisper_local',
    llm_provider TEXT NOT NULL DEFAULT 'llama_local',
    prompt_template_used TEXT
);

CREATE INDEX IF NOT EXISTS idx_meetings_started_at ON meetings(started_at);
CREATE INDEX IF NOT EXISTS idx_meetings_status ON meetings(status);
CREATE INDEX IF NOT EXISTS idx_meetings_sync_status ON meetings(sync_status);
CREATE INDEX IF NOT EXISTS idx_meetings_folder_id ON meetings(folder_id);

CREATE TABLE IF NOT EXISTS transcript_segments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id TEXT NOT NULL,
    start_ms INTEGER NOT NULL,
    end_ms INTEGER NOT NULL,
    text TEXT NOT NULL,
    speaker_id TEXT,
    confidence REAL NOT NULL DEFAULT 0.0,
    is_final INTEGER NOT NULL DEFAULT 0,
    version INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_segments_meeting_id ON transcript_segments(meeting_id);
CREATE INDEX IF NOT EXISTS idx_segments_meeting_start ON transcript_segments(meeting_id, start_ms);

CREATE TABLE IF NOT EXISTS speakers (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    label TEXT NOT NULL,
    display_name TEXT,
    is_local_user INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS summaries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id TEXT NOT NULL,
    content TEXT NOT NULL,
    generation_mode TEXT NOT NULL,
    model_used TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    user_edited INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS action_items (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    description TEXT NOT NULL,
    assignee TEXT,
    deadline TEXT,
    source_segment_id INTEGER,
    status TEXT NOT NULL DEFAULT 'pending',
    loop_task_id TEXT,
    sync_status TEXT NOT NULL DEFAULT 'not_synced',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_action_items_meeting_id ON action_items(meeting_id);

CREATE TABLE IF NOT EXISTS decisions (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    description TEXT NOT NULL,
    context TEXT,
    participants TEXT NOT NULL DEFAULT '[]',
    source_segment_ids TEXT NOT NULL DEFAULT '[]',
    vault_entry_id TEXT,
    sync_status TEXT NOT NULL DEFAULT 'not_synced',
    created_at INTEGER NOT NULL,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS prompt_templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    content TEXT NOT NULL,
    meeting_type TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    is_custom INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000)
);

CREATE INDEX IF NOT EXISTS idx_prompt_templates_name ON prompt_templates(name);
CREATE INDEX IF NOT EXISTS idx_prompt_templates_meeting_type ON prompt_templates(meeting_type);

CREATE TABLE IF NOT EXISTS api_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider TEXT NOT NULL UNIQUE,
    key_encrypted TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000)
);

CREATE INDEX IF NOT EXISTS idx_api_keys_provider ON api_keys(provider);

CREATE TABLE IF NOT EXISTS configuration (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000)
);

CREATE VIRTUAL TABLE IF NOT EXISTS transcript_search USING fts5(
    text,
    content='transcript_segments',
    content_rowid='id',
    tokenize='porter unicode61 remove_diacritics 2'
);

-- Keep the external-content FTS index in sync with transcript_segments.
-- Without these triggers the index stays empty and MATCH returns nothing.
CREATE TRIGGER IF NOT EXISTS transcript_segments_ai AFTER INSERT ON transcript_segments BEGIN
    INSERT INTO transcript_search(rowid, text) VALUES (new.id, new.text);
END;
CREATE TRIGGER IF NOT EXISTS transcript_segments_ad AFTER DELETE ON transcript_segments BEGIN
    INSERT INTO transcript_search(transcript_search, rowid, text) VALUES ('delete', old.id, old.text);
END;
CREATE TRIGGER IF NOT EXISTS transcript_segments_au AFTER UPDATE OF text ON transcript_segments BEGIN
    INSERT INTO transcript_search(transcript_search, rowid, text) VALUES ('delete', old.id, old.text);
    INSERT INTO transcript_search(rowid, text) VALUES (new.id, new.text);
END;

CREATE TABLE IF NOT EXISTS sync_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    target TEXT NOT NULL,
    payload TEXT NOT NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    next_retry_at INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    last_error TEXT,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sync_queue_next_retry ON sync_queue(next_retry_at);
CREATE INDEX IF NOT EXISTS idx_sync_queue_status ON sync_queue(status);

-- Rebuild the FTS index from transcript_segments so rows inserted before the
-- triggers existed also become searchable. Idempotent.
INSERT INTO transcript_search(transcript_search) VALUES ('rebuild');
"#;
