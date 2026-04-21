use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use http_body_util::BodyExt;
use tower::ServiceExt;

use fudoki_backend::tts::{OpenAiCompatibleConfig, TtsConfig};

async fn spawn_fake_upstream(app: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}/v1")
}

#[tokio::test]
async fn speak_endpoint_returns_audio_bytes_from_upstream() {
    let upstream_url = spawn_fake_upstream(
        Router::new().route(
            "/v1/audio/speech",
            post(|| async {
                ([(header::CONTENT_TYPE, "audio/mpeg")], vec![1_u8, 2_u8, 3_u8]).into_response()
            }),
        ),
    )
    .await;

    let app = fudoki_backend::app::build_router_with_tts_config(TtsConfig::enabled(
        OpenAiCompatibleConfig {
            base_url: upstream_url,
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
                .method(Method::POST)
                .uri("/api/tts/speak")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"provider":"openai-compatible","text":"こんにちは","voice":"alloy","format":"mp3","speed":1.0}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "audio/mpeg"
    );
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.as_ref(), &[1_u8, 2_u8, 3_u8]);
}

#[tokio::test]
async fn speak_endpoint_returns_explicit_json_error_when_upstream_rejects_credentials() {
    let upstream_url = spawn_fake_upstream(
        Router::new().route(
            "/v1/audio/speech",
            post(|| async {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({
                        "error": { "message": "bad key" }
                    })),
                )
                    .into_response()
            }),
        ),
    )
    .await;

    let app = fudoki_backend::app::build_router_with_tts_config(TtsConfig::enabled(
        OpenAiCompatibleConfig {
            base_url: upstream_url,
            api_key: "bad-key".to_string(),
            model: "gpt-4o-mini-tts".to_string(),
            default_voice: "alloy".to_string(),
            default_format: "mp3".to_string(),
        },
        Some("openai-compatible".to_string()),
    ));

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tts/speak")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"provider":"openai-compatible","text":"こんにちは"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "tts_request_failed");
    assert_eq!(json["error"]["message"], "TTS request failed: 401 Unauthorized");
}

