use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;

use crate::analyzer::Analyzer;
use crate::dictionary::{DictionaryPayload, DictionaryService};
use crate::models::{AnalyzeRequest, AnalyzeResponse};

#[derive(Clone)]
pub struct AppState {
    pub analyzer: Arc<Analyzer>,
    pub dictionary: Arc<DictionaryService>,
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

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ready",
        tokenizer_ready: true,
        dictionary_ready: state.dictionary.is_ready(),
    })
}

async fn index() -> impl IntoResponse {
    Html(std::fs::read_to_string("index.html").expect("read index.html"))
}

async fn analyze(
    State(state): State<AppState>,
    Json(payload): Json<AnalyzeRequest>,
) -> Json<AnalyzeResponse> {
    Json(AnalyzeResponse {
        lines: state.analyzer.analyze(&payload.text),
    })
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

pub fn build_router() -> Router {
    let state = AppState {
        analyzer: Arc::new(Analyzer::new()),
        dictionary: Arc::new(DictionaryService::new()),
    };

    Router::new()
        .route("/", get(index))
        .route("/api/health", get(health))
        .route("/api/analyze", post(analyze))
        .route("/api/dictionary", get(dictionary_lookup))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state)
}
