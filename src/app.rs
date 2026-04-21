use std::path::PathBuf;

use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use tower_http::services::ServeDir;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    tokenizer_ready: bool,
    dictionary_ready: bool,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "starting",
        tokenizer_ready: false,
        dictionary_ready: false,
    })
}

async fn index() -> impl IntoResponse {
    Html(std::fs::read_to_string("index.html").expect("read index.html"))
}

pub fn build_router() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/api/health", get(health))
        .nest_service("/static", ServeDir::new(PathBuf::from("static")))
}
