use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    extract::Json as AxumJson,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use http_body_util::BodyExt;
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

use fudoki_backend::tts::{OpenAiCompatibleConfig, TtsConfig};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("fudoki-{prefix}-{stamp}"));
    fs::create_dir_all(&path).unwrap();
    path
}

fn build_test_router(tts: TtsConfig, prefix: &str) -> axum::Router {
    fudoki_backend::app::build_router_with_tts_config_and_data_dir(tts, unique_temp_dir(prefix))
}

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

    let app = build_test_router(TtsConfig::enabled(
        OpenAiCompatibleConfig {
            base_url: upstream_url,
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini-tts".to_string(),
            model_options: vec!["gpt-4o-mini-tts".to_string(), "gpt-audio-1.5".to_string()],
            default_voice: "alloy".to_string(),
            voice_options: vec!["alloy".to_string(), "marin".to_string()],
            default_format: "mp3".to_string(),
        },
        Some("openai-compatible".to_string()),
    ), "tts-audio-bytes");

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

    let app = build_test_router(TtsConfig::enabled(
        OpenAiCompatibleConfig {
            base_url: upstream_url,
            api_key: "bad-key".to_string(),
            model: "gpt-4o-mini-tts".to_string(),
            model_options: vec!["gpt-4o-mini-tts".to_string()],
            default_voice: "alloy".to_string(),
            voice_options: vec!["alloy".to_string()],
            default_format: "mp3".to_string(),
        },
        Some("openai-compatible".to_string()),
    ), "tts-upstream-error");

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

#[tokio::test]
async fn speak_endpoint_does_not_mark_provider_unavailable_after_bad_request() {
    let app = build_test_router(TtsConfig::enabled(
        OpenAiCompatibleConfig {
            base_url: "https://example.invalid/v1".to_string(),
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini-tts".to_string(),
            model_options: vec!["gpt-4o-mini-tts".to_string()],
            default_voice: "alloy".to_string(),
            voice_options: vec!["alloy".to_string()],
            default_format: "mp3".to_string(),
        },
        Some("openai-compatible".to_string()),
    ), "tts-bad-request");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tts/speak")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"provider":"system","text":"こんにちは"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let providers = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/tts/providers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(providers.status(), StatusCode::OK);

    let body = providers.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let providers = json["providers"].as_array().unwrap();
    assert_eq!(providers.len(), 2);
    assert_eq!(providers[1]["id"], "openai-compatible");
    assert_eq!(providers[1]["status"], "available");
}

#[tokio::test]
async fn speak_endpoint_forwards_selected_model_and_voice_to_upstream() {
    let captured = Arc::new(Mutex::new(None::<serde_json::Value>));
    let captured_clone = captured.clone();
    let upstream_url = spawn_fake_upstream(
        Router::new().route(
            "/v1/audio/speech",
            post(move |AxumJson(payload): AxumJson<serde_json::Value>| {
                let captured_clone = captured_clone.clone();
                async move {
                    *captured_clone.lock().unwrap() = Some(payload);
                    ([(header::CONTENT_TYPE, "audio/mpeg")], vec![9_u8, 8_u8]).into_response()
                }
            }),
        ),
    )
    .await;

    let app = build_test_router(TtsConfig::enabled(
        OpenAiCompatibleConfig {
            base_url: upstream_url,
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini-tts".to_string(),
            model_options: vec!["gpt-4o-mini-tts".to_string(), "gpt-audio-1.5".to_string()],
            default_voice: "alloy".to_string(),
            voice_options: vec!["alloy".to_string(), "marin".to_string()],
            default_format: "mp3".to_string(),
        },
        Some("openai-compatible".to_string()),
    ), "tts-forwards-selection");

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tts/speak")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"provider":"openai-compatible","text":"こんにちは","model":"gpt-audio-1.5","voice":"marin","format":"mp3","speed":1.2}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let payload = captured.lock().unwrap().clone().unwrap();
    assert_eq!(payload["model"], "gpt-audio-1.5");
    assert_eq!(payload["voice"], "marin");
    let speed = payload["speed"].as_f64().unwrap();
    assert!((speed - 1.2).abs() < 0.001, "unexpected speed: {speed}");
}

#[tokio::test]
async fn speak_endpoint_rejects_remote_model_not_in_allowed_options() {
    let app = build_test_router(TtsConfig::enabled(
        OpenAiCompatibleConfig {
            base_url: "https://example.invalid/v1".to_string(),
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini-tts".to_string(),
            model_options: vec!["gpt-4o-mini-tts".to_string()],
            default_voice: "alloy".to_string(),
            voice_options: vec!["alloy".to_string(), "marin".to_string()],
            default_format: "mp3".to_string(),
        },
        Some("openai-compatible".to_string()),
    ), "tts-reject-model");

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tts/speak")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"provider":"openai-compatible","text":"こんにちは","model":"gpt-audio-1.5"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "tts_bad_request");
}

#[tokio::test]
async fn speak_endpoint_rejects_remote_voice_not_in_allowed_options() {
    let app = build_test_router(TtsConfig::enabled(
        OpenAiCompatibleConfig {
            base_url: "https://example.invalid/v1".to_string(),
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini-tts".to_string(),
            model_options: vec!["gpt-4o-mini-tts".to_string()],
            default_voice: "alloy".to_string(),
            voice_options: vec!["alloy".to_string()],
            default_format: "mp3".to_string(),
        },
        Some("openai-compatible".to_string()),
    ), "tts-reject-voice");

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tts/speak")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"provider":"openai-compatible","text":"こんにちは","voice":"marin"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "tts_bad_request");
}

#[tokio::test]
async fn speak_endpoint_reuses_cached_audio_for_same_document_revision() {
    let request_count = Arc::new(AtomicUsize::new(0));
    let request_count_clone = request_count.clone();
    let upstream_url = spawn_fake_upstream(
        Router::new().route(
            "/v1/audio/speech",
            post(move || {
                let request_count_clone = request_count_clone.clone();
                async move {
                    request_count_clone.fetch_add(1, Ordering::SeqCst);
                    ([(header::CONTENT_TYPE, "audio/mpeg")], vec![4_u8, 2_u8]).into_response()
                }
            }),
        ),
    )
    .await;

    let app = build_test_router(TtsConfig::enabled(
        OpenAiCompatibleConfig {
            base_url: upstream_url,
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini-tts".to_string(),
            model_options: vec!["gpt-4o-mini-tts".to_string()],
            default_voice: "alloy".to_string(),
            voice_options: vec!["alloy".to_string()],
            default_format: "mp3".to_string(),
        },
        Some("openai-compatible".to_string()),
    ), "tts-cache-same-revision");

    let request_body = r#"{"provider":"openai-compatible","text":"こんにちは","voice":"alloy","format":"mp3","speed":1.0,"document_id":"doc-1","document_revision":1,"cache_scope_version":"openai-compatible|gpt-4o-mini-tts|alloy|mp3"}"#;

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tts/speak")
                .header("content-type", "application/json")
                .body(Body::from(request_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);

    let second = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tts/speak")
                .header("content-type", "application/json")
                .body(Body::from(request_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::OK);
    assert_eq!(request_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn speak_endpoint_evicts_only_changed_document_revision() {
    let request_count = Arc::new(AtomicUsize::new(0));
    let request_count_clone = request_count.clone();
    let upstream_url = spawn_fake_upstream(
        Router::new().route(
            "/v1/audio/speech",
            post(move || {
                let request_count_clone = request_count_clone.clone();
                async move {
                    request_count_clone.fetch_add(1, Ordering::SeqCst);
                    ([(header::CONTENT_TYPE, "audio/mpeg")], vec![7_u8, 1_u8]).into_response()
                }
            }),
        ),
    )
    .await;

    let app = build_test_router(TtsConfig::enabled(
        OpenAiCompatibleConfig {
            base_url: upstream_url,
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini-tts".to_string(),
            model_options: vec!["gpt-4o-mini-tts".to_string()],
            default_voice: "alloy".to_string(),
            voice_options: vec!["alloy".to_string()],
            default_format: "mp3".to_string(),
        },
        Some("openai-compatible".to_string()),
    ), "tts-evict-changed-revision");

    let doc1_rev1 = r#"{"provider":"openai-compatible","text":"こんにちは","voice":"alloy","format":"mp3","speed":1.0,"document_id":"doc-1","document_revision":1,"cache_scope_version":"openai-compatible|gpt-4o-mini-tts|alloy|mp3"}"#;
    let doc2_rev1 = r#"{"provider":"openai-compatible","text":"さようなら","voice":"alloy","format":"mp3","speed":1.0,"document_id":"doc-2","document_revision":1,"cache_scope_version":"openai-compatible|gpt-4o-mini-tts|alloy|mp3"}"#;
    let doc1_rev2 = r#"{"provider":"openai-compatible","text":"こんにちは、更新版","voice":"alloy","format":"mp3","speed":1.0,"document_id":"doc-1","document_revision":2,"cache_scope_version":"openai-compatible|gpt-4o-mini-tts|alloy|mp3"}"#;

    for body in [doc1_rev1, doc2_rev1, doc1_rev2, doc2_rev1] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/tts/speak")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    assert_eq!(request_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn speak_endpoint_clears_entire_cache_when_cache_scope_changes() {
    let request_count = Arc::new(AtomicUsize::new(0));
    let request_count_clone = request_count.clone();
    let upstream_url = spawn_fake_upstream(
        Router::new().route(
            "/v1/audio/speech",
            post(move || {
                let request_count_clone = request_count_clone.clone();
                async move {
                    request_count_clone.fetch_add(1, Ordering::SeqCst);
                    ([(header::CONTENT_TYPE, "audio/mpeg")], vec![8_u8, 5_u8]).into_response()
                }
            }),
        ),
    )
    .await;

    let app = build_test_router(TtsConfig::enabled(
        OpenAiCompatibleConfig {
            base_url: upstream_url,
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini-tts".to_string(),
            model_options: vec!["gpt-4o-mini-tts".to_string(), "gpt-audio-1.5".to_string()],
            default_voice: "alloy".to_string(),
            voice_options: vec!["alloy".to_string(), "marin".to_string()],
            default_format: "mp3".to_string(),
        },
        Some("openai-compatible".to_string()),
    ), "tts-clear-scope-change");

    let scope_a_doc1 = r#"{"provider":"openai-compatible","text":"こんにちは","model":"gpt-4o-mini-tts","voice":"alloy","format":"mp3","speed":1.0,"document_id":"doc-1","document_revision":1,"cache_scope_version":"openai-compatible|gpt-4o-mini-tts|alloy|mp3"}"#;
    let scope_a_doc2 = r#"{"provider":"openai-compatible","text":"さようなら","model":"gpt-4o-mini-tts","voice":"alloy","format":"mp3","speed":1.0,"document_id":"doc-2","document_revision":1,"cache_scope_version":"openai-compatible|gpt-4o-mini-tts|alloy|mp3"}"#;
    let scope_b_doc1 = r#"{"provider":"openai-compatible","text":"こんにちは","model":"gpt-audio-1.5","voice":"marin","format":"mp3","speed":1.0,"document_id":"doc-1","document_revision":1,"cache_scope_version":"openai-compatible|gpt-audio-1.5|marin|mp3"}"#;
    let scope_b_doc2 = r#"{"provider":"openai-compatible","text":"さようなら","model":"gpt-audio-1.5","voice":"marin","format":"mp3","speed":1.0,"document_id":"doc-2","document_revision":1,"cache_scope_version":"openai-compatible|gpt-audio-1.5|marin|mp3"}"#;

    for body in [scope_a_doc1, scope_a_doc2, scope_b_doc1, scope_b_doc2] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/tts/speak")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    assert_eq!(request_count.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn speak_endpoint_persists_remote_cache_metadata_and_audio_across_router_rebuild() {
    let dir = unique_temp_dir("tts-persistent-cache");

    let request_count = Arc::new(AtomicUsize::new(0));
    let request_count_clone = request_count.clone();
    let upstream_url = spawn_fake_upstream(
        Router::new().route(
            "/v1/audio/speech",
            post(move || {
                let request_count_clone = request_count_clone.clone();
                async move {
                    request_count_clone.fetch_add(1, Ordering::SeqCst);
                    ([(header::CONTENT_TYPE, "audio/mpeg")], vec![6_u8, 6_u8, 1_u8]).into_response()
                }
            }),
        ),
    )
    .await;

    let config = TtsConfig::enabled(
        OpenAiCompatibleConfig {
            base_url: upstream_url,
            api_key: "test-key".to_string(),
            model: "gpt-4o-mini-tts".to_string(),
            model_options: vec!["gpt-4o-mini-tts".to_string()],
            default_voice: "alloy".to_string(),
            voice_options: vec!["alloy".to_string()],
            default_format: "mp3".to_string(),
        },
        Some("openai-compatible".to_string()),
    );

    let request_body = r#"{"provider":"openai-compatible","text":"保存される読み上げ","voice":"alloy","format":"mp3","speed":1.0,"document_id":"doc-persist-1","document_revision":1,"cache_scope_version":"openai-compatible|gpt-4o-mini-tts|alloy|mp3"}"#;

    let first_app = fudoki_backend::app::build_router_with_tts_config_and_data_dir(
        config.clone(),
        dir.clone(),
    );
    let first = first_app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tts/speak")
                .header("content-type", "application/json")
                .body(Body::from(request_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);

    let rebuilt_app = fudoki_backend::app::build_router_with_tts_config_and_data_dir(config, dir.clone());
    let second = rebuilt_app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tts/speak")
                .header("content-type", "application/json")
                .body(Body::from(request_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::OK);

    assert_eq!(request_count.load(Ordering::SeqCst), 1);

    let db = Connection::open(dir.join("fudoki.db")).unwrap();
    let count: i64 = db
        .query_row("SELECT COUNT(*) FROM tts_audio_cache", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);

    let cache_dir = dir.join("tts-cache");
    let audio_files = fs::read_dir(cache_dir).unwrap().count();
    assert_eq!(audio_files, 1);
}
