use std::time::Duration;

use rusqlite::{params, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::models::AnalyzeResponse;

use super::db::{now_millis, AppDatabase};

const ANALYZER_VERSION: &str = "sudachi-v1";
const ANALYSIS_CACHE_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);
const ANALYSIS_CACHE_MAX_ENTRIES: usize = 512;

#[derive(Clone, Debug)]
pub struct AnalysisCacheRepository {
    db: AppDatabase,
}

impl AnalysisCacheRepository {
    pub fn new(db: AppDatabase) -> Self {
        Self { db }
    }

    pub fn get_cached(
        &self,
        document_id: &str,
        document_revision: i64,
        text: &str,
    ) -> rusqlite::Result<Option<AnalyzeResponse>> {
        let content_hash = content_hash(text);
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT cache_key, result_json
                FROM analysis_cache
                WHERE document_id = ?1
                  AND document_revision = ?2
                  AND content_hash = ?3
                  AND analyzer_version = ?4
                LIMIT 1
                "#,
            )?;
            let row = stmt
                .query_row(
                    params![document_id, document_revision, content_hash, ANALYZER_VERSION],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                        ))
                    },
                )
                .optional()?;

            match row {
                Some((cache_key, result_json)) => {
                    conn.execute(
                        "UPDATE analysis_cache SET last_used_at = ?2 WHERE cache_key = ?1",
                        params![cache_key, now_millis()],
                    )?;
                    let parsed = serde_json::from_str::<AnalyzeResponse>(&result_json)
                        .map_err(json_to_db_error)?;
                    Ok(Some(parsed))
                }
                None => Ok(None),
            }
        })
    }

    pub fn get_by_document_revision(
        &self,
        document_id: &str,
        document_revision: i64,
    ) -> rusqlite::Result<Option<AnalyzeResponse>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT cache_key, result_json
                FROM analysis_cache
                WHERE document_id = ?1
                  AND document_revision = ?2
                ORDER BY last_used_at DESC
                LIMIT 1
                "#,
            )?;
            let row = stmt
                .query_row(params![document_id, document_revision], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                    ))
                })
                .optional()?;

            match row {
                Some((cache_key, result_json)) => {
                    conn.execute(
                        "UPDATE analysis_cache SET last_used_at = ?2 WHERE cache_key = ?1",
                        params![cache_key, now_millis()],
                    )?;
                    let parsed = serde_json::from_str::<AnalyzeResponse>(&result_json)
                        .map_err(json_to_db_error)?;
                    Ok(Some(parsed))
                }
                None => Ok(None),
            }
        })
    }

    pub fn store(
        &self,
        document_id: &str,
        document_revision: i64,
        text: &str,
        result: &AnalyzeResponse,
    ) -> rusqlite::Result<()> {
        let content_hash = content_hash(text);
        let cache_key = cache_key(document_id, document_revision, &content_hash);
        let serialized = serde_json::to_string(result).map_err(json_to_db_error)?;
        let now = now_millis();

        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO analysis_cache (
                    cache_key,
                    document_id,
                    document_revision,
                    content_hash,
                    analyzer_version,
                    result_json,
                    created_at,
                    last_used_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT(cache_key) DO UPDATE SET
                    result_json = excluded.result_json,
                    last_used_at = excluded.last_used_at
                "#,
                params![
                    cache_key,
                    document_id,
                    document_revision,
                    content_hash,
                    ANALYZER_VERSION,
                    serialized,
                    now,
                    now,
                ],
            )?;

            // Prune and evict within the same connection so counts are accurate.
            let cutoff = now - ANALYSIS_CACHE_TTL.as_millis() as i64;
            conn.execute("DELETE FROM analysis_cache WHERE last_used_at < ?1", [cutoff])?;

            let count: i64 = conn.query_row("SELECT COUNT(*) FROM analysis_cache", [], |row| row.get(0))?;
            if count as usize > ANALYSIS_CACHE_MAX_ENTRIES {
                let overflow = count as usize - ANALYSIS_CACHE_MAX_ENTRIES;
                conn.execute(
                    "DELETE FROM analysis_cache WHERE cache_key IN (SELECT cache_key FROM analysis_cache ORDER BY last_used_at ASC LIMIT ?1)",
                    [overflow as i64],
                )?;
            }

            Ok(())
        })
    }

    pub fn clear_all(&self) -> rusqlite::Result<()> {
        self.db.with_connection(|conn| {
            conn.execute("DELETE FROM analysis_cache", [])?;
            Ok(())
        })
    }

}

fn cache_key(document_id: &str, document_revision: i64, content_hash: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(ANALYZER_VERSION.as_bytes());
    hasher.update(b"|");
    hasher.update(document_id.as_bytes());
    hasher.update(b"|");
    hasher.update(document_revision.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(content_hash.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn content_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn json_to_db_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}
