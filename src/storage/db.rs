use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

#[derive(Clone, Debug)]
pub struct AppDatabase {
    db_path: PathBuf,
}

impl AppDatabase {
    pub fn from_env() -> rusqlite::Result<Self> {
        let data_dir = match std::env::var("FUDOKI_DATA_DIR") {
            Ok(path) if !path.trim().is_empty() => PathBuf::from(path),
            _ => default_data_dir(),
        };
        Self::new(data_dir)
    }

    pub fn new(data_dir: PathBuf) -> rusqlite::Result<Self> {
        fs::create_dir_all(&data_dir)
            .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;

        let db = Self {
            db_path: data_dir.join("fudoki.db"),
        };
        db.initialize()?;
        Ok(db)
    }

    pub fn connect(&self) -> rusqlite::Result<Connection> {
        let conn = Connection::open(&self.db_path)?;
        configure_connection(&conn)?;
        Ok(conn)
    }

    pub fn with_connection<T, F>(&self, op: F) -> rusqlite::Result<T>
    where
        F: FnOnce(&mut Connection) -> rusqlite::Result<T>,
    {
        let mut conn = self.connect()?;
        op(&mut conn)
    }

    pub fn path(&self) -> &Path {
        &self.db_path
    }

    fn initialize(&self) -> rusqlite::Result<()> {
        self.with_connection(|conn| {
            conn.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS documents (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    title_mode TEXT NOT NULL,
                    content TEXT NOT NULL,
                    source_kind TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL,
                    revision INTEGER NOT NULL,
                    deleted_at INTEGER NULL
                );

                CREATE TABLE IF NOT EXISTS settings (
                    key TEXT PRIMARY KEY,
                    value_json TEXT NOT NULL,
                    updated_at INTEGER NOT NULL
                );

                CREATE TABLE IF NOT EXISTS analysis_cache (
                    cache_key TEXT PRIMARY KEY,
                    document_id TEXT NULL,
                    document_revision INTEGER NULL,
                    content_hash TEXT NOT NULL,
                    analyzer_version TEXT NOT NULL,
                    result_json TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    last_used_at INTEGER NOT NULL
                );

                CREATE TABLE IF NOT EXISTS tts_audio_cache (
                    cache_key TEXT PRIMARY KEY,
                    provider TEXT NOT NULL,
                    document_id TEXT NULL,
                    document_revision INTEGER NULL,
                    cache_scope_version TEXT NULL,
                    text_hash TEXT NOT NULL,
                    model TEXT NOT NULL,
                    voice TEXT NOT NULL,
                    format TEXT NOT NULL,
                    speed_hundredths INTEGER NOT NULL,
                    content_type TEXT NOT NULL,
                    audio_path TEXT NOT NULL,
                    expires_at INTEGER NOT NULL,
                    created_at INTEGER NOT NULL,
                    last_used_at INTEGER NOT NULL
                );

                CREATE TABLE IF NOT EXISTS app_meta (
                    key TEXT PRIMARY KEY,
                    value_json TEXT NOT NULL
                );
                "#,
            )?;
            Ok(())
        })
    }

    pub fn get_meta(&self, key: &str) -> rusqlite::Result<Option<String>> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT value_json FROM app_meta WHERE key = ?1")?;
            let mut rows = stmt.query([key])?;
            match rows.next()? {
                Some(row) => row.get(0).map(Some),
                None => Ok(None),
            }
        })
    }

    pub fn set_meta_json(&self, key: &str, value_json: &str) -> rusqlite::Result<()> {
        self.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO app_meta (key, value_json)
                VALUES (?1, ?2)
                ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json
                "#,
                (key, value_json),
            )?;
            Ok(())
        })
    }
}

pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn default_data_dir() -> PathBuf {
    let base = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    base.join("fudoki")
}

fn configure_connection(conn: &Connection) -> rusqlite::Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(())
}
