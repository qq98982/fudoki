use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use crate::models::{
    CreateDocumentRequest, DocumentListResponse, DocumentPayload, LegacyBrowserDataRequest,
    TitleMode, UpdateDocumentRequest,
};

use super::db::{now_millis, AppDatabase};

const ACTIVE_DOCUMENT_META_KEY: &str = "active_document_id";
const LEGACY_IMPORT_META_KEY: &str = "legacy_import_completed";

#[derive(Clone, Debug)]
pub struct DocumentsRepository {
    db: AppDatabase,
}

#[derive(Debug)]
pub enum UpdateDocumentError {
    NotFound,
    Conflict,
    Db(rusqlite::Error),
}

impl From<rusqlite::Error> for UpdateDocumentError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Db(value)
    }
}

impl DocumentsRepository {
    pub fn new(db: AppDatabase) -> Self {
        Self { db }
    }

    pub fn list(&self) -> rusqlite::Result<DocumentListResponse> {
        let documents = self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, title, title_mode, content, source_kind, created_at, updated_at, revision
                FROM documents
                WHERE deleted_at IS NULL
                ORDER BY updated_at DESC, created_at DESC
                "#,
            )?;

            let rows = stmt.query_map([], |row| {
                Ok(DocumentPayload {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    title_mode: TitleMode::from_db_value(&row.get::<_, String>(2)?),
                    content: row.get(3)?,
                    source_kind: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    revision: row.get(7)?,
                })
            })?;

            rows.collect::<rusqlite::Result<Vec<_>>>()
        })?;

        let active_document_id = self.active_document_id()?;
        Ok(DocumentListResponse {
            documents,
            active_document_id,
        })
    }

    pub fn create(&self, request: CreateDocumentRequest) -> rusqlite::Result<DocumentPayload> {
        let now = now_millis();
        let id = Uuid::now_v7().to_string();
        let title_mode = request.title_mode.unwrap_or(TitleMode::Auto);
        let content = request.content;
        let title = resolve_title(request.title, &content, &title_mode);
        let source_kind = request.source_kind.unwrap_or_else(|| "user".to_string());

        let document = DocumentPayload {
            id: id.clone(),
            title,
            title_mode,
            content,
            source_kind,
            created_at: now,
            updated_at: now,
            revision: 1,
        };

        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO documents (
                    id, title, title_mode, content, source_kind, created_at, updated_at, revision, deleted_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)
                "#,
                params![
                    &document.id,
                    &document.title,
                    document.title_mode.as_db_value(),
                    &document.content,
                    &document.source_kind,
                    document.created_at,
                    document.updated_at,
                    document.revision,
                ],
            )?;
            Ok(())
        })?;

        self.set_active_document_id(Some(id))?;
        Ok(document)
    }

    pub fn update(
        &self,
        id: &str,
        request: UpdateDocumentRequest,
    ) -> Result<DocumentPayload, UpdateDocumentError> {
        let id_owned = id.to_string();
        let result: rusqlite::Result<Result<DocumentPayload, UpdateDocumentError>> =
            self.db.with_connection(|conn| {
                let tx = conn.unchecked_transaction()?;

                let existing = tx
                    .query_row(
                        r#"
                        SELECT id, title, title_mode, content, source_kind, created_at, updated_at, revision
                        FROM documents
                        WHERE id = ?1 AND deleted_at IS NULL
                        "#,
                        [&id_owned],
                        |row| {
                            Ok(DocumentPayload {
                                id: row.get(0)?,
                                title: row.get(1)?,
                                title_mode: TitleMode::from_db_value(&row.get::<_, String>(2)?),
                                content: row.get(3)?,
                                source_kind: row.get(4)?,
                                created_at: row.get(5)?,
                                updated_at: row.get(6)?,
                                revision: row.get(7)?,
                            })
                        },
                    )
                    .optional()?;

                let existing = match existing {
                    Some(doc) => doc,
                    None => return Ok(Err(UpdateDocumentError::NotFound)),
                };

                if existing.revision != request.expected_revision {
                    return Ok(Err(UpdateDocumentError::Conflict));
                }

                let title_mode = request.title_mode.unwrap_or(existing.title_mode.clone());
                let title = resolve_title(request.title, &request.content, &title_mode);
                let now = now_millis();
                let next_revision = existing.revision + 1;

                tx.execute(
                    r#"
                    UPDATE documents
                    SET title = ?2, title_mode = ?3, content = ?4, updated_at = ?5, revision = ?6
                    WHERE id = ?1 AND deleted_at IS NULL
                    "#,
                    params![
                        &existing.id,
                        &title,
                        title_mode.as_db_value(),
                        &request.content,
                        now,
                        next_revision,
                    ],
                )?;

                tx.commit()?;

                Ok(Ok(DocumentPayload {
                    id: existing.id,
                    title,
                    title_mode,
                    content: request.content,
                    source_kind: existing.source_kind,
                    created_at: existing.created_at,
                    updated_at: now,
                    revision: next_revision,
                }))
            });

        result.map_err(UpdateDocumentError::from)?
    }

    pub fn delete(&self, id: &str) -> rusqlite::Result<bool> {
        let id_owned = id.to_string();
        self.db.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;

            let affected = tx.execute(
                "UPDATE documents SET deleted_at = ?2 WHERE id = ?1 AND deleted_at IS NULL",
                params![&id_owned, now_millis()],
            )?;

            if affected > 0 {
                let active_id: Option<String> = tx
                    .query_row(
                        "SELECT value_json FROM app_meta WHERE key = 'active_document_id'",
                        [],
                        |row| row.get(0),
                    )
                    .optional()?
                    .and_then(|raw: String| serde_json::from_str(&raw).ok());

                if active_id.as_deref() == Some(&id_owned) {
                    let next_id: Option<String> = {
                        let mut stmt = tx.prepare(
                            "SELECT id FROM documents WHERE deleted_at IS NULL ORDER BY updated_at DESC LIMIT 1",
                        )?;
                        stmt.query_row([], |row| row.get(0)).optional()?
                    };

                    match next_id {
                        Some(next) => {
                            tx.execute(
                                "INSERT INTO app_meta (key, value_json) VALUES ('active_document_id', ?1) ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
                                params![serde_json::to_string(&next).unwrap()],
                            )?;
                        }
                        None => {
                            tx.execute("DELETE FROM app_meta WHERE key = 'active_document_id'", [])?;
                        }
                    }
                }

                tx.commit()?;
                Ok(true)
            } else {
                Ok(false)
            }
        })
    }

    pub fn duplicate(&self, id: &str) -> rusqlite::Result<DocumentPayload> {
        let existing = self.get(id)?.ok_or_else(|| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(
                std::io::Error::new(std::io::ErrorKind::NotFound, "document not found"),
            ))
        })?;
        let now = now_millis();
        let new_id = Uuid::now_v7().to_string();
        let title = format!("{} (copy)", existing.title);

        let document = DocumentPayload {
            id: new_id.clone(),
            title,
            title_mode: TitleMode::Custom,
            content: existing.content,
            source_kind: "user".to_string(),
            created_at: now,
            updated_at: now,
            revision: 1,
        };

        self.db.with_connection(|conn| {
            conn.execute(
                r#"
                INSERT INTO documents (
                    id, title, title_mode, content, source_kind, created_at, updated_at, revision, deleted_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)
                "#,
                params![
                    &document.id,
                    &document.title,
                    document.title_mode.as_db_value(),
                    &document.content,
                    &document.source_kind,
                    document.created_at,
                    document.updated_at,
                    document.revision,
                ],
            )?;
            Ok(())
        })?;

        self.set_active_document_id(Some(new_id))?;
        Ok(document)
    }

    pub fn import_legacy(&self, payload: LegacyBrowserDataRequest) -> rusqlite::Result<()> {
        self.db.with_connection(|conn| {
            let tx = conn.unchecked_transaction()?;
            tx.execute("DELETE FROM documents", [])?;

            for legacy in &payload.documents {
                let content = legacy.content.as_text();
                let title_mode = TitleMode::Auto;
                let title = derive_title(&content);
                let source_kind = "user";
                tx.execute(
                    r#"
                    INSERT INTO documents (
                        id, title, title_mode, content, source_kind, created_at, updated_at, revision, deleted_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, NULL)
                    "#,
                    params![
                        legacy.id,
                        title,
                        title_mode.as_db_value(),
                        content,
                        source_kind,
                        legacy.created_at.unwrap_or_else(now_millis),
                        legacy.updated_at.unwrap_or_else(now_millis),
                    ],
                )?;
            }

            tx.execute("DELETE FROM settings", [])?;
            for (key, value) in &payload.settings {
                let value_json = match serde_json::to_string(value) {
                    Ok(json) => json,
                    Err(err) => {
                        eprintln!("legacy import: skipping setting {key}: {err}");
                        continue;
                    }
                };
                tx.execute(
                    r#"
                    INSERT INTO settings (key, value_json, updated_at)
                    VALUES (?1, ?2, ?3)
                    ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at
                    "#,
                    params![key, value_json, now_millis()],
                )?;
            }

            let active_id = payload.active_id.or_else(|| payload.documents.first().map(|doc| doc.id.clone()));
            if let Some(active_id) = active_id {
                tx.execute(
                    r#"
                    INSERT INTO app_meta (key, value_json)
                    VALUES (?1, ?2)
                    ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json
                    "#,
                    params![ACTIVE_DOCUMENT_META_KEY, serde_json::to_string(&active_id).unwrap()],
                )?;
            }

            tx.execute(
                r#"
                INSERT INTO app_meta (key, value_json)
                VALUES (?1, ?2)
                ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json
                "#,
                params![LEGACY_IMPORT_META_KEY, "true"],
            )?;

            tx.commit()
        })
    }

    pub fn active_document_id(&self) -> rusqlite::Result<Option<String>> {
        let value_json = self.db.get_meta(ACTIVE_DOCUMENT_META_KEY)?;
        Ok(value_json.and_then(|value| serde_json::from_str::<String>(&value).ok()))
    }

    pub fn set_active_document_id(&self, id: Option<String>) -> rusqlite::Result<()> {
        match id {
            Some(id) => self
                .db
                .set_meta_json(ACTIVE_DOCUMENT_META_KEY, &serde_json::to_string(&id).unwrap()),
            None => self.db.with_connection(|conn| {
                conn.execute("DELETE FROM app_meta WHERE key = ?1", [ACTIVE_DOCUMENT_META_KEY])?;
                Ok(())
            }),
        }
    }

    fn get(&self, id: &str) -> rusqlite::Result<Option<DocumentPayload>> {
        self.db.with_connection(|conn| {
            conn.query_row(
                r#"
                SELECT id, title, title_mode, content, source_kind, created_at, updated_at, revision
                FROM documents
                WHERE id = ?1 AND deleted_at IS NULL
                "#,
                [id],
                |row| {
                    Ok(DocumentPayload {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        title_mode: TitleMode::from_db_value(&row.get::<_, String>(2)?),
                        content: row.get(3)?,
                        source_kind: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                        revision: row.get(7)?,
                    })
                },
            )
            .optional()
        })
    }
}

fn resolve_title(title: Option<String>, content: &str, title_mode: &TitleMode) -> String {
    match title_mode {
        TitleMode::Custom => title
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| derive_title(content)),
        TitleMode::Auto => derive_title(content),
    }
}

fn derive_title(content: &str) -> String {
    content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "Untitled".to_string())
}
