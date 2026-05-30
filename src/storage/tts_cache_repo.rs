use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{params, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::tts::{SpeakRequest, SynthesizedSpeech};

use super::db::{now_millis, AppDatabase};

const REMOTE_TTS_CACHE_TTL: Duration = Duration::from_secs(30 * 60);
const REMOTE_TTS_CACHE_MAX_ENTRIES: usize = 64;
const CACHE_SCOPE_META_KEY: &str = "tts_cache_scope_version";
const DOCUMENT_REVISION_META_PREFIX: &str = "tts_document_revision:";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TtsCacheKey {
    pub provider: String,
    pub document_id: Option<String>,
    pub text: String,
    pub model: String,
    pub voice: String,
    pub format: String,
    pub speed_hundredths: u16,
}

impl TtsCacheKey {
    pub fn stable_key(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.provider.as_bytes());
        hasher.update(b"|");
        if let Some(document_id) = &self.document_id {
            hasher.update(document_id.as_bytes());
        }
        hasher.update(b"|");
        hasher.update(self.text.as_bytes());
        hasher.update(b"|");
        hasher.update(self.model.as_bytes());
        hasher.update(b"|");
        hasher.update(self.voice.as_bytes());
        hasher.update(b"|");
        hasher.update(self.format.as_bytes());
        hasher.update(b"|");
        hasher.update(self.speed_hundredths.to_string().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn text_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.text.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[derive(Clone, Debug)]
pub struct TtsCacheRepository {
    db: AppDatabase,
    cache_dir: PathBuf,
}

impl TtsCacheRepository {
    pub fn new(db: AppDatabase) -> rusqlite::Result<Self> {
        let cache_dir = db
            .path()
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("tts-cache");
        fs::create_dir_all(&cache_dir).map_err(io_to_db_error)?;
        Ok(Self { db, cache_dir })
    }

    pub fn prepare_request_scope(&self, request: &SpeakRequest) -> rusqlite::Result<()> {
        if let Some(next_scope) = request.cache_scope_version.as_deref() {
            let stored = self
                .db
                .get_meta(CACHE_SCOPE_META_KEY)?
                .and_then(|value| serde_json::from_str::<String>(&value).ok());
            if stored.as_deref().is_some_and(|current| current != next_scope) {
                self.clear_all()?;
            }
            self.db
                .set_meta_json(CACHE_SCOPE_META_KEY, &serde_json::to_string(next_scope).unwrap())?;
        }

        if let (Some(document_id), Some(document_revision)) =
            (request.document_id.as_deref(), request.document_revision)
        {
            let key = format!("{DOCUMENT_REVISION_META_PREFIX}{document_id}");
            let stored = self
                .db
                .get_meta(&key)?
                .and_then(|value| serde_json::from_str::<u64>(&value).ok());
            if stored.is_some_and(|current| current != document_revision) {
                self.clear_document(document_id)?;
            }
            self.db
                .set_meta_json(&key, &serde_json::to_string(&document_revision).unwrap())?;
        }

        self.prune_expired()
    }

    pub fn get(&self, key: &TtsCacheKey) -> rusqlite::Result<Option<SynthesizedSpeech>> {
        self.prune_expired()?;
        let stable_key = key.stable_key();
        let row = self.db.with_connection(|conn| {
            conn.query_row(
                r#"
                SELECT audio_path, content_type
                FROM tts_audio_cache
                WHERE cache_key = ?1 AND expires_at > ?2
                "#,
                params![stable_key, now_millis()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
        })?;

        match row {
            Some((audio_path, content_type)) => {
                let bytes = fs::read(self.cache_dir.join(&audio_path)).map_err(io_to_db_error)?;
                self.db.with_connection(|conn| {
                    conn.execute(
                        "UPDATE tts_audio_cache SET last_used_at = ?2 WHERE cache_key = ?1",
                        params![stable_key, now_millis()],
                    )?;
                    Ok(())
                })?;
                Ok(Some(SynthesizedSpeech { content_type, bytes }))
            }
            None => Ok(None),
        }
    }

    pub fn insert(
        &self,
        key: &TtsCacheKey,
        request: &SpeakRequest,
        speech: &SynthesizedSpeech,
    ) -> rusqlite::Result<()> {
        self.prune_expired()?;
        let stable_key = key.stable_key();
        let extension = extension_for_format(&key.format);
        let audio_path = format!("{stable_key}.{extension}");
        fs::write(self.cache_dir.join(&audio_path), &speech.bytes).map_err(io_to_db_error)?;

        let now = now_millis();
        let expires_at = now + REMOTE_TTS_CACHE_TTL.as_millis() as i64;
        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO tts_audio_cache (
                    cache_key,
                    provider,
                    document_id,
                    document_revision,
                    cache_scope_version,
                    text_hash,
                    model,
                    voice,
                    format,
                    speed_hundredths,
                    content_type,
                    audio_path,
                    expires_at,
                    created_at,
                    last_used_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                ON CONFLICT(cache_key) DO UPDATE SET
                    document_revision = excluded.document_revision,
                    cache_scope_version = excluded.cache_scope_version,
                    content_type = excluded.content_type,
                    audio_path = excluded.audio_path,
                    expires_at = excluded.expires_at,
                    last_used_at = excluded.last_used_at
                "#,
                params![
                    stable_key,
                    key.provider,
                    key.document_id,
                    request.document_revision.map(|value| value as i64),
                    request.cache_scope_version,
                    key.text_hash(),
                    key.model,
                    key.voice,
                    key.format,
                    key.speed_hundredths as i64,
                    speech.content_type,
                    audio_path,
                    expires_at,
                    now,
                    now,
                ],
            )?;
            Ok(())
        })?;

        self.evict_lru_if_needed()
    }

    fn prune_expired(&self) -> rusqlite::Result<()> {
        let expired = self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT cache_key, audio_path FROM tts_audio_cache WHERE expires_at <= ?1",
            )?;
            let rows = stmt.query_map([now_millis()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })?;
        if expired.is_empty() {
            return Ok(());
        }
        self.remove_entries(&expired)
    }

    fn evict_lru_if_needed(&self) -> rusqlite::Result<()> {
        let count = self.db.with_connection(|conn| {
            conn.query_row("SELECT COUNT(*) FROM tts_audio_cache", [], |row| row.get::<_, i64>(0))
        })? as usize;

        if count <= REMOTE_TTS_CACHE_MAX_ENTRIES {
            return Ok(());
        }

        let overflow = count - REMOTE_TTS_CACHE_MAX_ENTRIES;
        let oldest = self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT cache_key, audio_path FROM tts_audio_cache ORDER BY last_used_at ASC LIMIT ?1",
            )?;
            let rows = stmt.query_map([overflow as i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })?;
        self.remove_entries(&oldest)
    }

    fn clear_all(&self) -> rusqlite::Result<()> {
        let all = self.db.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT cache_key, audio_path FROM tts_audio_cache")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })?;

        self.remove_entries(&all)?;
        self.db.with_connection(|conn| {
            conn.execute(
                "DELETE FROM app_meta WHERE key = ?1 OR key LIKE ?2",
                params![CACHE_SCOPE_META_KEY, format!("{DOCUMENT_REVISION_META_PREFIX}%")],
            )?;
            Ok(())
        })
    }

    fn clear_document(&self, document_id: &str) -> rusqlite::Result<()> {
        let rows = self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT cache_key, audio_path FROM tts_audio_cache WHERE document_id = ?1",
            )?;
            let rows = stmt.query_map([document_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })?;
        self.remove_entries(&rows)
    }

    fn remove_entries(&self, entries: &[(String, String)]) -> rusqlite::Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        // Delete DB rows first inside a transaction. Stale audio files left
        // on disk after a crash are harmless — no DB row points to them.
        self.db.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            for (cache_key, _) in entries {
                tx.execute("DELETE FROM tts_audio_cache WHERE cache_key = ?1", [cache_key])?;
            }
            tx.commit()
        })?;

        for (_, audio_path) in entries {
            let full_path = self.cache_dir.join(audio_path);
            let _ = fs::remove_file(&full_path);
        }

        Ok(())
    }
}

fn extension_for_format(format: &str) -> &str {
    match format {
        "wav" => "wav",
        "opus" => "opus",
        "flac" => "flac",
        _ => "mp3",
    }
}

fn io_to_db_error(error: std::io::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}
