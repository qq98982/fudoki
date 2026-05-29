use std::sync::Arc;
use std::path::PathBuf;

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;
use tower_http::services::ServeFile;

use crate::analyzer::Analyzer;
use crate::dictionary::{DictionaryPayload, DictionaryService};
use crate::models::{
    AnalyzeRequest, AnalyzeResponse, ApiErrorBody, ApiErrorResponse, CreateDocumentRequest,
    DocumentEnvelope, DocumentListResponse, LegacyBrowserDataRequest, MigrationResponse,
    SettingsResponse, UpdateDocumentRequest, UpdateSettingsRequest,
};
use crate::storage::db::AppDatabase;
use crate::storage::analysis_cache_repo::AnalysisCacheRepository;
use crate::storage::documents_repo::{DocumentsRepository, UpdateDocumentError};
use crate::storage::settings_repo::SettingsRepository;
use crate::tts::{SpeakRequest, TtsConfig, TtsProvidersResponse};

#[derive(Clone)]
pub struct AppState {
    pub analyzer: Arc<Analyzer>,
    pub dictionary: Arc<DictionaryService>,
    pub tts: TtsConfig,
    pub analysis_cache: AnalysisCacheRepository,
    pub documents: DocumentsRepository,
    pub settings: SettingsRepository,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    tokenizer_ready: bool,
    dictionary_ready: bool,
}

#[derive(Deserialize)]
struct DictionaryQuery {
    term: String,
}

#[derive(Deserialize)]
struct AnalysisQuery {
    revision: i64,
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ready",
        tokenizer_ready: true,
        dictionary_ready: state.dictionary.is_ready(),
    })
}

async fn index() -> impl IntoResponse {
    let index_path = frontend_dist_path("index.html");
    let fallback_path = PathBuf::from("index.html");
    let html = std::fs::read_to_string(&index_path)
        .or_else(|_| std::fs::read_to_string(&fallback_path))
        .expect("read frontend index");
    Html(html)
}

async fn login() -> impl IntoResponse {
    Html(std::fs::read_to_string("login.html").expect("read login.html"))
}

async fn analyze(
    State(state): State<AppState>,
    Json(payload): Json<AnalyzeRequest>,
) -> Json<AnalyzeResponse> {
    if let (Some(document_id), Some(document_revision)) =
        (payload.document_id.as_deref(), payload.document_revision)
    {
        if let Ok(Some(cached)) = state
            .analysis_cache
            .get_cached(document_id, document_revision, &payload.text)
        {
            return Json(cached);
        }

        let response = AnalyzeResponse {
            lines: state.analyzer.analyze(&payload.text),
        };
        let _ = state
            .analysis_cache
            .store(document_id, document_revision, &payload.text, &response);
        return Json(response);
    }

    Json(AnalyzeResponse {
        lines: state.analyzer.analyze(&payload.text),
    })
}

async fn cached_analysis(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<AnalysisQuery>,
) -> Result<Json<AnalyzeResponse>, StatusCode> {
    state
        .analysis_cache
        .get_by_document_revision(&id, query.revision)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn clear_analysis_cache(
    State(state): State<AppState>,
) -> Result<StatusCode, (StatusCode, Json<ApiErrorResponse>)> {
    state
        .analysis_cache
        .clear_all()
        .map_err(internal_error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn dictionary_lookup(
    State(state): State<AppState>,
    Query(query): Query<DictionaryQuery>,
) -> Result<Json<DictionaryPayload>, StatusCode> {
    state
        .dictionary
        .lookup(&query.term)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn tts_providers(State(state): State<AppState>) -> Json<TtsProvidersResponse> {
    Json(state.tts.providers_response())
}

async fn tts_speak(
    State(state): State<AppState>,
    Json(payload): Json<SpeakRequest>,
) -> impl IntoResponse {
    match state.tts.synthesize(&payload).await {
        Ok(speech) => ([(header::CONTENT_TYPE, speech.content_type)], speech.bytes).into_response(),
        Err((status, err)) => (status, Json(err)).into_response(),
    }
}

async fn list_documents(
    State(state): State<AppState>,
) -> Result<Json<DocumentListResponse>, (StatusCode, Json<ApiErrorResponse>)> {
    state.documents.list().map(Json).map_err(internal_error)
}

async fn create_document(
    State(state): State<AppState>,
    Json(payload): Json<CreateDocumentRequest>,
) -> Result<(StatusCode, Json<DocumentEnvelope>), (StatusCode, Json<ApiErrorResponse>)> {
    let document = state.documents.create(payload).map_err(internal_error)?;
    Ok((StatusCode::CREATED, Json(DocumentEnvelope { document })))
}

async fn update_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateDocumentRequest>,
) -> Result<Json<DocumentEnvelope>, (StatusCode, Json<ApiErrorResponse>)> {
    let document = state.documents.update(&id, payload).map_err(document_update_error)?;
    Ok(Json(DocumentEnvelope { document }))
}

async fn delete_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ApiErrorResponse>)> {
    let deleted = state.documents.delete(&id).map_err(internal_error)?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found("document_not_found", "Document not found"))
    }
}

async fn duplicate_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<DocumentEnvelope>), (StatusCode, Json<ApiErrorResponse>)> {
    let document = state.documents.duplicate(&id).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("not found") {
            not_found("document_not_found", "Document not found")
        } else {
            internal_error(e)
        }
    })?;
    Ok((StatusCode::CREATED, Json(DocumentEnvelope { document })))
}

async fn get_settings(
    State(state): State<AppState>,
) -> Result<Json<SettingsResponse>, (StatusCode, Json<ApiErrorResponse>)> {
    state.settings.get_all().map(Json).map_err(internal_error)
}

async fn update_settings(
    State(state): State<AppState>,
    Json(payload): Json<UpdateSettingsRequest>,
) -> Result<Json<SettingsResponse>, (StatusCode, Json<ApiErrorResponse>)> {
    state.settings.update(payload).map(Json).map_err(internal_error)
}

async fn import_legacy_browser_data(
    State(state): State<AppState>,
    Json(payload): Json<LegacyBrowserDataRequest>,
) -> Result<Json<MigrationResponse>, (StatusCode, Json<ApiErrorResponse>)> {
    let imported_documents = payload.documents.len();
    state
        .documents
        .import_legacy(payload)
        .map_err(internal_error)?;
    Ok(Json(MigrationResponse { imported_documents }))
}

pub fn build_router_with_tts_config(tts: TtsConfig) -> Router {
    let db = AppDatabase::from_env().expect("initialize app database");
    build_router_with_tts_config_and_database(tts, db)
}

pub fn build_router_with_tts_config_and_data_dir(tts: TtsConfig, data_dir: PathBuf) -> Router {
    let db = AppDatabase::new(data_dir).expect("initialize app database");
    build_router_with_tts_config_and_database(tts, db)
}

fn build_router_with_tts_config_and_database(tts: TtsConfig, db: AppDatabase) -> Router {
    let tts = tts.with_persistent_cache(db.clone());
    let state = AppState {
        analyzer: Arc::new(Analyzer::new()),
        dictionary: Arc::new(DictionaryService::new()),
        tts,
        analysis_cache: AnalysisCacheRepository::new(db.clone()),
        documents: DocumentsRepository::new(db.clone()),
        settings: SettingsRepository::new(db),
    };

    Router::new()
        .route("/", get(index))
        .route("/login.html", get(login))
        .route_service("/favicon.svg", ServeFile::new(frontend_dist_path("favicon.svg")))
        .route("/api/health", get(health))
        .route("/api/analyze", post(analyze))
        .route("/api/analysis-cache", axum::routing::delete(clear_analysis_cache))
        .route("/api/dictionary", get(dictionary_lookup))
        .route("/api/documents", get(list_documents).post(create_document))
        .route("/api/documents/{id}", put(update_document).delete(delete_document))
        .route("/api/documents/{id}/duplicate", post(duplicate_document))
        .route("/api/documents/{id}/analysis", get(cached_analysis))
        .route("/api/settings", get(get_settings).put(update_settings))
        .route(
            "/api/migrations/legacy-browser-data",
            post(import_legacy_browser_data),
        )
        .route("/api/tts/providers", get(tts_providers))
        .route("/api/tts/speak", post(tts_speak))
        .nest_service("/assets", ServeDir::new(frontend_dist_path("assets")))
        .nest_service("/static", ServeDir::new("static"))
        .fallback(get(index))
        .with_state(state)
}

pub fn build_router() -> Router {
    build_router_with_tts_config(TtsConfig::from_env())
}

fn internal_error(error: rusqlite::Error) -> (StatusCode, Json<ApiErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiErrorResponse {
            error: ApiErrorBody {
                code: "internal_error".to_string(),
                message: format!("Database error: {error}"),
            },
        }),
    )
}

fn not_found(code: &str, message: &str) -> (StatusCode, Json<ApiErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiErrorResponse {
            error: ApiErrorBody {
                code: code.to_string(),
                message: message.to_string(),
            },
        }),
    )
}

fn document_update_error(
    error: UpdateDocumentError,
) -> (StatusCode, Json<ApiErrorResponse>) {
    match error {
        UpdateDocumentError::NotFound => not_found("document_not_found", "Document not found"),
        UpdateDocumentError::Conflict => (
            StatusCode::CONFLICT,
            Json(ApiErrorResponse {
                error: ApiErrorBody {
                    code: "document_revision_conflict".to_string(),
                    message: "Document revision conflict".to_string(),
                },
            }),
        ),
        UpdateDocumentError::Db(error) => internal_error(error),
    }
}

fn frontend_dist_path(relative: &str) -> PathBuf {
    PathBuf::from("frontend").join("dist").join(relative)
}
