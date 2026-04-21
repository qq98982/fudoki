use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use tower::ServiceExt;

use fudoki_backend::tts::{OpenAiCompatibleConfig, TtsConfig};

#[tokio::test]
async fn providers_endpoint_lists_only_system_when_online_tts_is_not_configured() {
    let app = fudoki_backend::app::build_router_with_tts_config(TtsConfig::disabled());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/tts/providers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let providers = json["providers"].as_array().unwrap();

    assert_eq!(json["default_provider"], "system");
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0]["id"], "system");
    assert_eq!(providers[0]["status"], "available");
}

#[tokio::test]
async fn providers_endpoint_exposes_online_provider_defaults_when_configured() {
    let app = fudoki_backend::app::build_router_with_tts_config(TtsConfig::enabled(
        OpenAiCompatibleConfig {
            base_url: "https://example.invalid/v1".to_string(),
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini-tts".to_string(),
            default_voice: "alloy".to_string(),
            default_format: "mp3".to_string(),
        },
        Some("openai-compatible".to_string()),
    ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/tts/providers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let providers = json["providers"].as_array().unwrap();

    assert_eq!(json["default_provider"], "openai-compatible");
    assert_eq!(providers.len(), 2);
    assert_eq!(providers[1]["id"], "openai-compatible");
    assert_eq!(providers[1]["status"], "available");
    assert_eq!(providers[1]["defaults"]["voice"], "alloy");
    assert_eq!(providers[1]["defaults"]["format"], "mp3");
}

