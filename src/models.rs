use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub text: String,
    #[serde(default)]
    pub document_id: Option<String>,
    #[serde(default)]
    pub document_revision: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenPayload {
    pub surface: String,
    pub lemma: String,
    pub reading: String,
    pub tts_text: String,
    pub pos: Vec<String>,
    pub source: String,
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyzeResponse {
    pub lines: Vec<Vec<TokenPayload>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TitleMode {
    Auto,
    Custom,
}

impl TitleMode {
    pub fn as_db_value(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Custom => "custom",
        }
    }

    pub fn from_db_value(value: &str) -> Self {
        match value {
            "custom" => Self::Custom,
            _ => Self::Auto,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentPayload {
    pub id: String,
    pub title: String,
    pub title_mode: TitleMode,
    pub content: String,
    pub source_kind: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub revision: i64,
}

#[derive(Debug, Serialize)]
pub struct DocumentEnvelope {
    pub document: DocumentPayload,
}

#[derive(Debug, Serialize)]
pub struct DocumentListResponse {
    pub documents: Vec<DocumentPayload>,
    pub active_document_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDocumentRequest {
    pub content: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub title_mode: Option<TitleMode>,
    #[serde(default)]
    pub source_kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDocumentRequest {
    pub content: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub title_mode: Option<TitleMode>,
    pub expected_revision: i64,
}

#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    pub values: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub values: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct LegacyBrowserDataRequest {
    pub documents: Vec<LegacyDocumentPayload>,
    #[serde(default, alias = "activeId")]
    pub active_id: Option<String>,
    #[serde(default)]
    pub settings: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct LegacyDocumentPayload {
    pub id: String,
    pub content: LegacyDocumentContent,
    #[serde(default, alias = "createdAt")]
    pub created_at: Option<i64>,
    #[serde(default, alias = "updatedAt")]
    pub updated_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum LegacyDocumentContent {
    Text(String),
    Lines(Vec<String>),
}

impl LegacyDocumentContent {
    pub fn as_text(&self) -> String {
        match self {
            Self::Text(value) => value.clone(),
            Self::Lines(lines) => lines.join("\n"),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MigrationResponse {
    pub imported_documents: usize,
}

#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ApiErrorResponse {
    pub error: ApiErrorBody,
}
