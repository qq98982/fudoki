use std::sync::Arc;

use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use tower_http::services::ServeDir;

use crate::analyzer::Analyzer;
use crate::models::{AnalyzeRequest, AnalyzeResponse};

#[derive(Clone)]
pub struct AppState {
    pub analyzer: Arc<Analyzer>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    tokenizer_ready: bool,
    dictionary_ready: bool,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ready",
        tokenizer_ready: true,
        dictionary_ready: false,
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

pub fn build_router() -> Router {
    let state = AppState {
        analyzer: Arc::new(Analyzer::new()),
    };

    Router::new()
        .route("/", get(index))
        .route("/api/health", get(health))
        .route("/api/analyze", post(analyze))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state)
}
