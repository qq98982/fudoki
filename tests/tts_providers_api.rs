use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use tower::ServiceExt;

use fudoki_backend::tts::{OpenAiCompatibleConfig, TtsConfig};

use std::sync::{Mutex, OnceLock};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvGuard {
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn new(keys: &[&'static str]) -> Self {
        let saved = keys
            .iter()
            .map(|k| (*k, std::env::var(k).ok()))
            .collect::<Vec<_>>();
        Self { saved }
    }

    fn set(&self, key: &'static str, value: &str) {
        std::env::set_var(key, value);
    }

    fn remove(&self, key: &'static str) {
        std::env::remove_var(key);
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, v) in self.saved.drain(..) {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
    }
}

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

#[tokio::test]
async fn from_env_reads_voice_and_format_keys_not_default_voice_and_format() {
    let _guard = env_lock().lock().unwrap();

    let keys = [
        "FUDOKI_TTS_OPENAI_BASE_URL",
        "FUDOKI_TTS_OPENAI_API_KEY",
        "FUDOKI_TTS_OPENAI_MODEL",
        "FUDOKI_TTS_OPENAI_VOICE",
        "FUDOKI_TTS_OPENAI_FORMAT",
        // Legacy keys that must NOT be read.
        "FUDOKI_TTS_OPENAI_DEFAULT_VOICE",
        "FUDOKI_TTS_OPENAI_DEFAULT_FORMAT",
        "FUDOKI_TTS_DEFAULT_PROVIDER",
    ];

    let env = EnvGuard::new(&keys);
    for k in keys {
        env.remove(k);
    }

    env.set("FUDOKI_TTS_OPENAI_BASE_URL", "https://example.invalid/v1");
    env.set("FUDOKI_TTS_OPENAI_API_KEY", "test-key");
    env.set("FUDOKI_TTS_OPENAI_MODEL", "gpt-4o-mini-tts");
    env.set("FUDOKI_TTS_OPENAI_VOICE", "voice-from-env");
    env.set("FUDOKI_TTS_OPENAI_FORMAT", "format-from-env");

    // If the implementation mistakenly reads these, the test will fail.
    env.set("FUDOKI_TTS_OPENAI_DEFAULT_VOICE", "wrong-voice");
    env.set("FUDOKI_TTS_OPENAI_DEFAULT_FORMAT", "wrong-format");

    let app = fudoki_backend::app::build_router_with_tts_config(TtsConfig::from_env());

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

    assert_eq!(providers.len(), 2);
    assert_eq!(providers[1]["id"], "openai-compatible");
    assert_eq!(providers[1]["defaults"]["voice"], "voice-from-env");
    assert_eq!(providers[1]["defaults"]["format"], "format-from-env");
}
