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
            model_options: vec!["gpt-4o-mini-tts".to_string(), "gpt-audio-1.5".to_string()],
            default_voice: "alloy".to_string(),
            voice_options: vec!["alloy".to_string(), "marin".to_string()],
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
    assert_eq!(providers[1]["defaults"]["model"], "gpt-4o-mini-tts");
    assert_eq!(providers[1]["defaults"]["voice"], "alloy");
    assert_eq!(providers[1]["defaults"]["format"], "mp3");
    assert_eq!(providers[1]["options"]["models"][0], "gpt-4o-mini-tts");
    assert_eq!(providers[1]["options"]["models"][1], "gpt-audio-1.5");
    assert_eq!(providers[1]["options"]["voices"][0], "alloy");
    assert_eq!(providers[1]["options"]["voices"][1], "marin");
}

#[tokio::test]
async fn from_env_reads_voice_and_format_keys_not_default_voice_and_format() {
    let _guard = env_lock().lock().unwrap();

    let keys = [
        "FUDOKI_TTS_OPENAI_BASE_URL",
        "FUDOKI_TTS_OPENAI_API_KEY",
        "FUDOKI_TTS_OPENAI_MODEL",
        "FUDOKI_TTS_OPENAI_MODEL_OPTIONS",
        "FUDOKI_TTS_OPENAI_VOICE",
        "FUDOKI_TTS_OPENAI_VOICE_OPTIONS",
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
    env.set("FUDOKI_TTS_OPENAI_MODEL_OPTIONS", "gpt-4o-mini-tts, gpt-audio-1.5");
    env.set("FUDOKI_TTS_OPENAI_VOICE", "voice-from-env");
    env.set("FUDOKI_TTS_OPENAI_VOICE_OPTIONS", "voice-from-env, marin");
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
    assert_eq!(providers[1]["defaults"]["model"], "gpt-4o-mini-tts");
    assert_eq!(providers[1]["defaults"]["voice"], "voice-from-env");
    assert_eq!(providers[1]["defaults"]["format"], "format-from-env");
    assert_eq!(providers[1]["options"]["models"][0], "gpt-4o-mini-tts");
    assert_eq!(providers[1]["options"]["models"][1], "gpt-audio-1.5");
    assert_eq!(providers[1]["options"]["voices"][0], "voice-from-env");
    assert_eq!(providers[1]["options"]["voices"][1], "marin");
}

#[tokio::test]
async fn from_env_requires_model_to_enable_online_provider() {
    let _guard = env_lock().lock().unwrap();

    let keys = [
        "FUDOKI_TTS_OPENAI_BASE_URL",
        "FUDOKI_TTS_OPENAI_API_KEY",
        "FUDOKI_TTS_OPENAI_MODEL",
        "FUDOKI_TTS_OPENAI_MODEL_OPTIONS",
        "FUDOKI_TTS_OPENAI_VOICE",
        "FUDOKI_TTS_OPENAI_VOICE_OPTIONS",
        "FUDOKI_TTS_OPENAI_FORMAT",
        "FUDOKI_TTS_DEFAULT_PROVIDER",
    ];

    let env = EnvGuard::new(&keys);
    for k in keys {
        env.remove(k);
    }

    env.set("FUDOKI_TTS_OPENAI_BASE_URL", "https://example.invalid/v1");
    env.set("FUDOKI_TTS_OPENAI_API_KEY", "test-key");
    // Intentionally omit FUDOKI_TTS_OPENAI_MODEL.
    env.set("FUDOKI_TTS_OPENAI_VOICE", "voice-from-env");
    env.set("FUDOKI_TTS_OPENAI_FORMAT", "format-from-env");

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

    assert_eq!(json["default_provider"], "system");
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0]["id"], "system");
    assert_eq!(providers[0]["status"], "available");
}

#[tokio::test]
async fn from_env_clamps_invalid_default_provider_when_online_is_configured() {
    let _guard = env_lock().lock().unwrap();

    let keys = [
        "FUDOKI_TTS_OPENAI_BASE_URL",
        "FUDOKI_TTS_OPENAI_API_KEY",
        "FUDOKI_TTS_OPENAI_MODEL",
        "FUDOKI_TTS_OPENAI_MODEL_OPTIONS",
        "FUDOKI_TTS_OPENAI_VOICE",
        "FUDOKI_TTS_OPENAI_VOICE_OPTIONS",
        "FUDOKI_TTS_OPENAI_FORMAT",
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
    env.set("FUDOKI_TTS_DEFAULT_PROVIDER", "not-a-real-provider");

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
    assert_eq!(json["default_provider"], "openai-compatible");
}

#[tokio::test]
async fn from_env_requires_nonempty_base_url_api_key_and_model() {
    let _guard = env_lock().lock().unwrap();

    let keys = [
        "FUDOKI_TTS_OPENAI_BASE_URL",
        "FUDOKI_TTS_OPENAI_API_KEY",
        "FUDOKI_TTS_OPENAI_MODEL",
        "FUDOKI_TTS_OPENAI_MODEL_OPTIONS",
        "FUDOKI_TTS_OPENAI_VOICE",
        "FUDOKI_TTS_OPENAI_VOICE_OPTIONS",
        "FUDOKI_TTS_OPENAI_FORMAT",
        "FUDOKI_TTS_DEFAULT_PROVIDER",
    ];

    let env = EnvGuard::new(&keys);
    for k in keys {
        env.remove(k);
    }

    env.set("FUDOKI_TTS_OPENAI_BASE_URL", "https://example.invalid/v1");
    env.set("FUDOKI_TTS_OPENAI_API_KEY", "test-key");
    env.set("FUDOKI_TTS_OPENAI_MODEL", "gpt-4o-mini-tts");

    // Each of these should disable the online provider when blank/whitespace.
    for (key, value) in [
        ("FUDOKI_TTS_OPENAI_BASE_URL", "   "),
        ("FUDOKI_TTS_OPENAI_API_KEY", ""),
        ("FUDOKI_TTS_OPENAI_MODEL", "\n\t"),
    ] {
        env.set(key, value);

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

        assert_eq!(json["default_provider"], "system");
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0]["id"], "system");

        // Restore valid value for the next iteration.
        match key {
            "FUDOKI_TTS_OPENAI_BASE_URL" => env.set(key, "https://example.invalid/v1"),
            "FUDOKI_TTS_OPENAI_API_KEY" => env.set(key, "test-key"),
            "FUDOKI_TTS_OPENAI_MODEL" => env.set(key, "gpt-4o-mini-tts"),
            _ => unreachable!(),
        }
    }
}

#[tokio::test]
async fn from_env_treats_blank_voice_and_format_as_unset_and_uses_defaults() {
    let _guard = env_lock().lock().unwrap();

    let keys = [
        "FUDOKI_TTS_OPENAI_BASE_URL",
        "FUDOKI_TTS_OPENAI_API_KEY",
        "FUDOKI_TTS_OPENAI_MODEL",
        "FUDOKI_TTS_OPENAI_MODEL_OPTIONS",
        "FUDOKI_TTS_OPENAI_VOICE",
        "FUDOKI_TTS_OPENAI_VOICE_OPTIONS",
        "FUDOKI_TTS_OPENAI_FORMAT",
        "FUDOKI_TTS_DEFAULT_PROVIDER",
    ];

    let env = EnvGuard::new(&keys);
    for k in keys {
        env.remove(k);
    }

    env.set("FUDOKI_TTS_OPENAI_BASE_URL", "https://example.invalid/v1");
    env.set("FUDOKI_TTS_OPENAI_API_KEY", "test-key");
    env.set("FUDOKI_TTS_OPENAI_MODEL", "gpt-4o-mini-tts");
    env.set("FUDOKI_TTS_OPENAI_VOICE", "   ");
    env.set("FUDOKI_TTS_OPENAI_FORMAT", "\n\t");

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
    assert_eq!(providers[1]["defaults"]["voice"], "alloy");
    assert_eq!(providers[1]["defaults"]["format"], "mp3");
}

#[tokio::test]
async fn from_env_uses_trimmed_model_and_voice_option_lists() {
    let _guard = env_lock().lock().unwrap();

    let keys = [
        "FUDOKI_TTS_OPENAI_BASE_URL",
        "FUDOKI_TTS_OPENAI_API_KEY",
        "FUDOKI_TTS_OPENAI_MODEL",
        "FUDOKI_TTS_OPENAI_MODEL_OPTIONS",
        "FUDOKI_TTS_OPENAI_VOICE",
        "FUDOKI_TTS_OPENAI_VOICE_OPTIONS",
        "FUDOKI_TTS_OPENAI_FORMAT",
        "FUDOKI_TTS_DEFAULT_PROVIDER",
    ];

    let env = EnvGuard::new(&keys);
    for k in keys {
        env.remove(k);
    }

    env.set("FUDOKI_TTS_OPENAI_BASE_URL", "https://example.invalid/v1");
    env.set("FUDOKI_TTS_OPENAI_API_KEY", "test-key");
    env.set("FUDOKI_TTS_OPENAI_MODEL", "gpt-4o-mini-tts");
    env.set(
        "FUDOKI_TTS_OPENAI_MODEL_OPTIONS",
        " gpt-4o-mini-tts , , gpt-audio-1.5 , gpt-realtime-1.5 ",
    );
    env.set("FUDOKI_TTS_OPENAI_VOICE", "marin");
    env.set(
        "FUDOKI_TTS_OPENAI_VOICE_OPTIONS",
        " marin , cedar ,, alloy ",
    );

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
    let provider = &json["providers"][1];

    assert_eq!(provider["defaults"]["model"], "gpt-4o-mini-tts");
    assert_eq!(provider["defaults"]["voice"], "marin");
    assert_eq!(provider["options"]["models"][0], "gpt-4o-mini-tts");
    assert_eq!(provider["options"]["models"][1], "gpt-audio-1.5");
    assert_eq!(provider["options"]["models"][2], "gpt-realtime-1.5");
    assert_eq!(provider["options"]["voices"][0], "marin");
    assert_eq!(provider["options"]["voices"][1], "cedar");
    assert_eq!(provider["options"]["voices"][2], "alloy");
}

#[tokio::test]
#[cfg(debug_assertions)]
async fn providers_endpoint_fails_closed_when_last_error_mutex_is_poisoned() {
    let tts = TtsConfig::enabled(
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
    );

    tts.__poison_last_error_mutex_for_test();

    let app = fudoki_backend::app::build_router_with_tts_config(tts);

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
    assert_eq!(providers[1]["status"], "unavailable");
}
