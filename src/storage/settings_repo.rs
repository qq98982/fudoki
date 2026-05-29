use std::collections::HashMap;

use rusqlite::params;

use crate::models::{SettingsResponse, UpdateSettingsRequest};

use super::db::{now_millis, AppDatabase};

#[derive(Clone, Debug)]
pub struct SettingsRepository {
    db: AppDatabase,
}

impl SettingsRepository {
    pub fn new(db: AppDatabase) -> Self {
        Self { db }
    }

    pub fn get_all(&self) -> rusqlite::Result<SettingsResponse> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT key, value_json FROM settings ORDER BY key ASC")?;
            let rows = stmt.query_map([], |row| {
                let key: String = row.get(0)?;
                let raw: String = row.get(1)?;
                let value = serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
                Ok((key, value))
            })?;

            let values = rows.collect::<rusqlite::Result<HashMap<_, _>>>()?;
            Ok(SettingsResponse { values })
        })
    }

    pub fn update(&self, request: UpdateSettingsRequest) -> rusqlite::Result<SettingsResponse> {
        self.db.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            let now = now_millis();
            for (key, value) in request.values {
                tx.execute(
                    r#"
                    INSERT INTO settings (key, value_json, updated_at)
                    VALUES (?1, ?2, ?3)
                    ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at
                    "#,
                    params![key, serde_json::to_string(&value).unwrap(), now],
                )?;
            }
            tx.commit()
        })?;

        self.get_all()
    }
}
