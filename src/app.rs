use axum::{routing::get, Json, Router};
use serde::Serialize;

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

pub fn build_router() -> Router {
    Router::new().route("/api/health", get(health))
}
