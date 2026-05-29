# Fudoki OpenAI-Compatible TTS Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional `.env`-driven OpenAI-compatible online TTS provider that coexists with the existing system Web Speech path, defaults to `.env` values when configured, and surfaces configuration or request errors clearly.

**Architecture:** Keep the current browser `speechSynthesis` provider as `system`. Add a backend-backed `openai-compatible` provider exposed through `/api/tts/providers` and `/api/tts/speak`, with the frontend selecting the provider at runtime and using a dedicated remote audio player for line and full-text playback. Use injected backend config for deterministic tests and keep remote playback state separate from system speech state.

**Tech Stack:** Rust, Cargo, axum, tokio, reqwest, dotenvy, serde, Node built-in test runner, existing static frontend JS

---

## Planned File Structure

**Create**

- `src/tts.rs`
- `tests/tts_providers_api.rs`
- `tests/tts_speak_api.rs`
- `static/js/remote-tts.js`
- `tests/frontend/tts-provider-api.test.mjs`
- `tests/frontend/remote-tts.test.mjs`
- `tests/frontend/tts-provider-ui.test.mjs`
- `.env.example`

**Modify**

- `Cargo.toml`
- `.gitignore`
- `src/lib.rs`
- `src/main.rs`
- `src/app.rs`
- `static/js/backend-api.js`
- `static/main-js.js`
- `static/js/i18n.js`
- `static/styles.css`
- `index.html`
- `README.md`

## Task 1: Add TTS Config Model and Provider Metadata Endpoint

**Files:**
- Create: `src/tts.rs`
- Create: `tests/tts_providers_api.rs`
- Modify: `Cargo.toml`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Write the failing provider-metadata tests**

```rust
// tests/tts_providers_api.rs
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test tts_providers_api`

Expected: FAIL with missing `tts` module, missing `build_router_with_tts_config`, or missing `/api/tts/providers`

- [ ] **Step 3: Write the minimal backend config and provider endpoint**

```toml
# Cargo.toml
[dependencies]
axum = "0.8.8"
tokio = { version = "1.48.0", features = ["macros", "rt-multi-thread"] }
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.145"
tower = { version = "0.5.2", features = ["util"] }
tower-http = { version = "0.6.8", features = ["fs"] }
sudachi = { git = "https://github.com/WorksApplications/sudachi.rs.git", package = "sudachi" }
dotenvy = "0.15.7"
reqwest = { version = "0.12.24", default-features = false, features = ["json", "rustls-tls"] }
```

```rust
// src/lib.rs
pub mod analyzer;
pub mod app;
pub mod dictionary;
pub mod english;
pub mod models;
pub mod tts;
```

```rust
// src/main.rs
use fudoki_backend::app::build_router;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .expect("bind listener");
    axum::serve(listener, build_router())
        .await
        .expect("serve application");
}
```

```rust
// src/tts.rs
use std::sync::{Arc, Mutex};

use serde::Serialize;

#[derive(Clone, Debug)]
pub struct OpenAiCompatibleConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub default_voice: String,
    pub default_format: String,
}

#[derive(Clone, Debug)]
pub struct TtsConfig {
    pub default_provider: String,
    pub openai_compatible: Option<OpenAiCompatibleConfig>,
    last_error: Arc<Mutex<Option<String>>>,
}

#[derive(Serialize)]
pub struct TtsProviderDefaults {
    pub voice: String,
    pub format: String,
}

#[derive(Serialize)]
pub struct TtsProviderView {
    pub id: String,
    pub label: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults: Option<TtsProviderDefaults>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct TtsProvidersResponse {
    pub default_provider: String,
    pub providers: Vec<TtsProviderView>,
}

impl TtsConfig {
    pub fn disabled() -> Self {
        Self {
            default_provider: "system".to_string(),
            openai_compatible: None,
            last_error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn enabled(openai_compatible: OpenAiCompatibleConfig, default_provider: Option<String>) -> Self {
        Self {
            default_provider: default_provider.unwrap_or_else(|| "openai-compatible".to_string()),
            openai_compatible: Some(openai_compatible),
            last_error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn from_env() -> Self {
        let base_url = std::env::var("FUDOKI_TTS_OPENAI_BASE_URL").ok();
        let api_key = std::env::var("FUDOKI_TTS_OPENAI_API_KEY").ok();
        let model = std::env::var("FUDOKI_TTS_OPENAI_MODEL").ok();

        match (base_url, api_key, model) {
            (Some(base_url), Some(api_key), Some(model)) => Self::enabled(
                OpenAiCompatibleConfig {
                    base_url,
                    api_key,
                    model,
                    default_voice: std::env::var("FUDOKI_TTS_OPENAI_VOICE")
                        .unwrap_or_else(|_| "alloy".to_string()),
                    default_format: std::env::var("FUDOKI_TTS_OPENAI_FORMAT")
                        .unwrap_or_else(|_| "mp3".to_string()),
                },
                std::env::var("FUDOKI_TTS_DEFAULT_PROVIDER").ok(),
            ),
            _ => Self::disabled(),
        }
    }

    pub fn providers_response(&self) -> TtsProvidersResponse {
        let mut providers = vec![TtsProviderView {
            id: "system".to_string(),
            label: "System".to_string(),
            status: "available".to_string(),
            defaults: None,
            error: None,
        }];

        if let Some(config) = &self.openai_compatible {
            let error = self.last_error.lock().unwrap().clone();
            providers.push(TtsProviderView {
                id: "openai-compatible".to_string(),
                label: "OpenAI-compatible".to_string(),
                status: if error.is_some() {
                    "request_failed".to_string()
                } else {
                    "available".to_string()
                },
                defaults: Some(TtsProviderDefaults {
                    voice: config.default_voice.clone(),
                    format: config.default_format.clone(),
                }),
                error,
            });
        }

        TtsProvidersResponse {
            default_provider: if self.openai_compatible.is_some() {
                self.default_provider.clone()
            } else {
                "system".to_string()
            },
            providers,
        }
    }

    pub fn set_last_error(&self, message: Option<String>) {
        *self.last_error.lock().unwrap() = message;
    }
}
```

```rust
// src/app.rs
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
use crate::tts::{TtsConfig, TtsProvidersResponse};

#[derive(Clone)]
pub struct AppState {
    pub analyzer: Arc<Analyzer>,
    pub dictionary: Arc<DictionaryService>,
    pub tts: TtsConfig,
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

async fn tts_providers(State(state): State<AppState>) -> Json<TtsProvidersResponse> {
    Json(state.tts.providers_response())
}

async fn index() -> impl IntoResponse {
    Html(std::fs::read_to_string("index.html").expect("read index.html"))
}

async fn login() -> impl IntoResponse {
    Html(std::fs::read_to_string("login.html").expect("read login.html"))
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
    build_router_with_tts_config(TtsConfig::from_env())
}

pub fn build_router_with_tts_config(tts: TtsConfig) -> Router {
    let state = AppState {
        analyzer: Arc::new(Analyzer::new()),
        dictionary: Arc::new(DictionaryService::new()),
        tts,
    };

    Router::new()
        .route("/", get(index))
        .route("/login.html", get(login))
        .route("/api/health", get(health))
        .route("/api/analyze", post(analyze))
        .route("/api/dictionary", get(dictionary_lookup))
        .route("/api/tts/providers", get(tts_providers))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test tts_providers_api`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/lib.rs src/main.rs src/app.rs src/tts.rs tests/tts_providers_api.rs
git commit -m "feat: expose tts provider metadata endpoint"
```

## Task 2: Add Remote Speech Proxy Endpoint and Upstream Error Mapping

**Files:**
- Create: `tests/tts_speak_api.rs`
- Modify: `src/tts.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Write the failing speak-endpoint tests**

```rust
// tests/tts_speak_api.rs
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
                (
                    [(header::CONTENT_TYPE, "audio/mpeg")],
                    vec![1_u8, 2_u8, 3_u8],
                )
                    .into_response()
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
                .body(Body::from(
                    r#"{"provider":"openai-compatible","text":"こんにちは"}"#,
                ))
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test tts_speak_api`

Expected: FAIL with missing `/api/tts/speak`, missing request model, or missing upstream proxy logic

- [ ] **Step 3: Write the minimal upstream proxy and error normalization**

```rust
// src/tts.rs
use axum::http::{header, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SpeakRequest {
    pub provider: String,
    pub text: String,
    pub voice: Option<String>,
    pub format: Option<String>,
    pub speed: Option<f32>,
}

#[derive(Serialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct ApiErrorResponse {
    pub error: ApiErrorBody,
}

pub struct SynthesizedSpeech {
    pub content_type: String,
    pub bytes: Vec<u8>,
}

impl TtsConfig {
    pub async fn synthesize(&self, request: &SpeakRequest) -> Result<SynthesizedSpeech, (StatusCode, ApiErrorResponse)> {
        if request.provider != "openai-compatible" {
            return Err((
                StatusCode::BAD_REQUEST,
                ApiErrorResponse {
                    error: ApiErrorBody {
                        code: "unsupported_provider".to_string(),
                        message: "Unsupported TTS provider".to_string(),
                    },
                },
            ));
        }

        let config = self.openai_compatible.as_ref().ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                ApiErrorResponse {
                    error: ApiErrorBody {
                        code: "tts_not_configured".to_string(),
                        message: "OpenAI-compatible TTS is not configured".to_string(),
                    },
                },
            )
        })?;

        let response = reqwest::Client::new()
            .post(format!("{}/audio/speech", config.base_url.trim_end_matches('/')))
            .bearer_auth(&config.api_key)
            .json(&serde_json::json!({
                "model": config.model,
                "input": request.text,
                "voice": request.voice.clone().unwrap_or_else(|| config.default_voice.clone()),
                "response_format": request.format.clone().unwrap_or_else(|| config.default_format.clone()),
                "speed": request.speed.unwrap_or(1.0),
            }))
            .send()
            .await
            .map_err(|_| {
                self.set_last_error(Some("TTS request failed: network error".to_string()));
                (
                    StatusCode::BAD_GATEWAY,
                    ApiErrorResponse {
                        error: ApiErrorBody {
                            code: "tts_request_failed".to_string(),
                            message: "TTS request failed: network error".to_string(),
                        },
                    },
                )
            })?;

        if !response.status().is_success() {
            let message = format!("TTS request failed: {}", response.status());
            self.set_last_error(Some(message.clone()));
            return Err((
                StatusCode::BAD_GATEWAY,
                ApiErrorResponse {
                    error: ApiErrorBody {
                        code: "tts_request_failed".to_string(),
                        message,
                    },
                },
            ));
        }

        self.set_last_error(None);

        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("audio/mpeg")
            .to_string();
        let bytes = response.bytes().await.map_err(|_| {
            (
                StatusCode::BAD_GATEWAY,
                ApiErrorResponse {
                    error: ApiErrorBody {
                        code: "tts_request_failed".to_string(),
                        message: "TTS request failed: invalid audio body".to_string(),
                    },
                },
            )
        })?.to_vec();

        Ok(SynthesizedSpeech { content_type, bytes })
    }
}
```

```rust
// src/app.rs
use axum::{body::Body, http::header, response::Response};

use crate::tts::{ApiErrorResponse, SpeakRequest, SynthesizedSpeech, TtsConfig, TtsProvidersResponse};

async fn tts_speak(
    State(state): State<AppState>,
    Json(payload): Json<SpeakRequest>,
) -> Result<Response, (StatusCode, Json<ApiErrorResponse>)> {
    let SynthesizedSpeech { content_type, bytes } = state
        .tts
        .synthesize(&payload)
        .await
        .map_err(|(status, error)| (status, Json(error)))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(bytes))
        .unwrap())
}

pub fn build_router_with_tts_config(tts: TtsConfig) -> Router {
    let state = AppState {
        analyzer: Arc::new(Analyzer::new()),
        dictionary: Arc::new(DictionaryService::new()),
        tts,
    };

    Router::new()
        .route("/", get(index))
        .route("/login.html", get(login))
        .route("/api/health", get(health))
        .route("/api/analyze", post(analyze))
        .route("/api/dictionary", get(dictionary_lookup))
        .route("/api/tts/providers", get(tts_providers))
        .route("/api/tts/speak", post(tts_speak))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test tts_speak_api`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/tts.rs tests/tts_speak_api.rs
git commit -m "feat: proxy openai-compatible speech requests"
```

## Task 3: Add Frontend Backend API Helpers for TTS Providers and Speech Requests

**Files:**
- Create: `tests/frontend/tts-provider-api.test.mjs`
- Modify: `static/js/backend-api.js`

- [ ] **Step 1: Write the failing frontend API-helper tests**

```javascript
// tests/frontend/tts-provider-api.test.mjs
import test from "node:test";
import assert from "node:assert/strict";

import {
  fetchTtsProviders,
  requestRemoteSpeech,
} from "../../static/js/backend-api.js";

test("backend api exports tts helpers", () => {
  assert.equal(typeof fetchTtsProviders, "function");
  assert.equal(typeof requestRemoteSpeech, "function");
});

test("fetchTtsProviders reads provider metadata from the backend", async () => {
  let requestedUrl = null;
  const fetcher = async (url) => {
    requestedUrl = url;
    return {
      ok: true,
      json: async () => ({ default_provider: "system", providers: [{ id: "system" }] }),
    };
  };

  const result = await fetchTtsProviders(fetcher);
  assert.equal(requestedUrl, "/api/tts/providers");
  assert.equal(result.default_provider, "system");
  assert.equal(result.providers[0].id, "system");
});

test("requestRemoteSpeech posts provider payload and returns the response", async () => {
  let request = null;
  const response = {
    ok: true,
    headers: { get: (key) => (key === "content-type" ? "audio/mpeg" : null) },
    arrayBuffer: async () => new Uint8Array([1, 2, 3]).buffer,
  };
  const fetcher = async (url, init) => {
    request = { url, init };
    return response;
  };

  const result = await requestRemoteSpeech(
    { provider: "openai-compatible", text: "こんにちは", voice: "alloy", format: "mp3", speed: 1.0 },
    fetcher,
  );

  assert.equal(result, response);
  assert.equal(request.url, "/api/tts/speak");
  assert.equal(request.init.method, "POST");
  assert.equal(request.init.headers["Content-Type"], "application/json");
  assert.equal(
    request.init.body,
    JSON.stringify({
      provider: "openai-compatible",
      text: "こんにちは",
      voice: "alloy",
      format: "mp3",
      speed: 1.0,
    }),
  );
});

test("requestRemoteSpeech surfaces backend error messages", async () => {
  const fetcher = async () => ({
    ok: false,
    json: async () => ({
      error: {
        code: "tts_request_failed",
        message: "TTS request failed: 401 Unauthorized",
      },
    }),
  });

  await assert.rejects(
    () => requestRemoteSpeech({ provider: "openai-compatible", text: "こんにちは" }, fetcher),
    /TTS request failed: 401 Unauthorized/,
  );
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test tests/frontend/tts-provider-api.test.mjs`

Expected: FAIL with missing helper exports

- [ ] **Step 3: Write the minimal frontend API helpers**

```javascript
// static/js/backend-api.js
export function resolveTtsText(token) {
  if (token && typeof token.tts_text === "string" && token.tts_text.length > 0) {
    return token.tts_text;
  }
  if (token && typeof token.reading === "string" && token.reading.length > 0) {
    return token.reading;
  }
  if (token && typeof token.surface === "string") {
    return token.surface;
  }
  return "";
}

function delay(ms) {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

export async function waitForBackendReady({ fetcher = fetch, retries = 6, delayMs = 500 } = {}) {
  let lastStatus = { status: "starting" };
  for (let attempt = 0; attempt < retries; attempt += 1) {
    try {
      const response = await fetcher("/api/health");
      if (response.ok) {
        lastStatus = await response.json();
        if (lastStatus.status === "ready") {
          return lastStatus;
        }
      }
    } catch (_error) {
      // Keep retrying until attempts are exhausted.
    }

    if (attempt < retries - 1) {
      await delay(delayMs * (attempt + 1));
    }
  }

  return lastStatus;
}

export async function analyzeTextRequest(text, fetcher = fetch) {
  const response = await fetcher("/api/analyze", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ text }),
  });
  if (!response.ok) {
    throw new Error(`analyze failed: ${response.status}`);
  }
  return response.json();
}

export async function lookupDictionaryRequest(term, fetcher = fetch) {
  const response = await fetcher(`/api/dictionary?term=${encodeURIComponent(term)}`);
  if (response.status === 404) {
    return null;
  }
  if (!response.ok) {
    throw new Error(`dictionary failed: ${response.status}`);
  }
  return response.json();
}

export async function fetchTtsProviders(fetcher = fetch) {
  const response = await fetcher("/api/tts/providers");
  if (!response.ok) {
    throw new Error(`tts providers failed: ${response.status}`);
  }
  return response.json();
}

export async function requestRemoteSpeech(payload, fetcher = fetch) {
  const response = await fetcher("/api/tts/speak", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    let message = `tts speak failed: ${response.status}`;
    try {
      const body = await response.json();
      if (body && body.error && typeof body.error.message === "string") {
        message = body.error.message;
      }
    } catch (_error) {
      // Fall back to the generic message.
    }
    throw new Error(message);
  }

  return response;
}

if (typeof window !== "undefined") {
  window.FudokiBackendApi = {
    resolveTtsText,
    waitForBackendReady,
    analyzeTextRequest,
    lookupDictionaryRequest,
    fetchTtsProviders,
    requestRemoteSpeech,
  };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test tests/frontend/tts-provider-api.test.mjs`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add static/js/backend-api.js tests/frontend/tts-provider-api.test.mjs
git commit -m "feat: add frontend tts backend api helpers"
```

## Task 4: Create a Dedicated Remote Audio Player Module

**Files:**
- Create: `static/js/remote-tts.js`
- Create: `tests/frontend/remote-tts.test.mjs`
- Modify: `index.html`

- [ ] **Step 1: Write the failing remote-player tests**

```javascript
// tests/frontend/remote-tts.test.mjs
import test from "node:test";
import assert from "node:assert/strict";

import { createRemoteTtsPlayer } from "../../static/js/remote-tts.js";

function createFakeAudio(log) {
  return {
    src: "",
    paused: true,
    currentTime: 0,
    onended: null,
    onerror: null,
    async play() {
      log.push(["play", this.src]);
      this.paused = false;
    },
    pause() {
      log.push(["pause", this.src]);
      this.paused = true;
    },
  };
}

test("remote player creates object urls and transitions into playing state", async () => {
  const log = [];
  const revoked = [];
  const states = [];
  const player = createRemoteTtsPlayer({
    audioFactory: () => createFakeAudio(log),
    urlApi: {
      createObjectURL: () => "blob:test-1",
      revokeObjectURL: (value) => revoked.push(value),
    },
    onStateChange: (state) => states.push(state),
  });

  await player.playResponse({
    headers: { get: () => "audio/mpeg" },
    arrayBuffer: async () => new Uint8Array([1, 2, 3]).buffer,
  });

  assert.deepEqual(states, ["loading", "playing"]);
  assert.deepEqual(log, [["play", "blob:test-1"]]);
  assert.deepEqual(revoked, []);
});

test("remote player stop revokes object urls and returns to idle", async () => {
  const revoked = [];
  const player = createRemoteTtsPlayer({
    audioFactory: () => createFakeAudio([]),
    urlApi: {
      createObjectURL: () => "blob:test-2",
      revokeObjectURL: (value) => revoked.push(value),
    },
    onStateChange: () => {},
  });

  await player.playResponse({
    headers: { get: () => "audio/mpeg" },
    arrayBuffer: async () => new Uint8Array([4, 5, 6]).buffer,
  });

  player.stop();

  assert.deepEqual(revoked, ["blob:test-2"]);
  assert.equal(player.getState(), "idle");
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test tests/frontend/remote-tts.test.mjs`

Expected: FAIL with missing `remote-tts.js` module or missing `createRemoteTtsPlayer`

- [ ] **Step 3: Write the minimal remote audio player and load it in the page**

```javascript
// static/js/remote-tts.js
export function createRemoteTtsPlayer({
  audioFactory = () => new Audio(),
  blobFactory = (parts, options) => new Blob(parts, options),
  urlApi = URL,
  onStateChange = () => {},
} = {}) {
  let audio = null;
  let objectUrl = null;
  let state = "idle";

  function setState(nextState) {
    state = nextState;
    onStateChange(nextState);
  }

  function cleanupObjectUrl() {
    if (objectUrl) {
      urlApi.revokeObjectURL(objectUrl);
      objectUrl = null;
    }
  }

  function stop() {
    if (audio) {
      audio.pause();
      audio.src = "";
      audio.currentTime = 0;
    }
    cleanupObjectUrl();
    setState("idle");
  }

  async function playResponse(response) {
    stop();
    setState("loading");

    const buffer = await response.arrayBuffer();
    const contentType = response.headers.get("content-type") || "audio/mpeg";
    const blob = blobFactory([buffer], { type: contentType });
    objectUrl = urlApi.createObjectURL(blob);
    audio = audioFactory();
    audio.src = objectUrl;
    audio.onended = () => {
      cleanupObjectUrl();
      setState("idle");
    };
    audio.onerror = () => {
      cleanupObjectUrl();
      setState("error");
    };
    await audio.play();
    setState("playing");
  }

  return {
    getState: () => state,
    playResponse,
    stop,
  };
}

if (typeof window !== "undefined") {
  window.FudokiRemoteTts = { createRemoteTtsPlayer };
}
```

```html
<!-- index.html -->
<script type="module" src="static/js/backend-api.js"></script>
<script type="module" src="static/js/remote-tts.js"></script>
<script src="static/js/tts.js"></script>
<script src="static/js/i18n.js"></script>
<script src="static/main-js.js"></script>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test tests/frontend/remote-tts.test.mjs`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add static/js/remote-tts.js index.html tests/frontend/remote-tts.test.mjs
git commit -m "feat: add remote audio player for online tts"
```

## Task 5: Wire Provider Selection, Status UI, and Playback Dispatch

**Files:**
- Create: `tests/frontend/tts-provider-ui.test.mjs`
- Modify: `static/main-js.js`
- Modify: `static/js/i18n.js`
- Modify: `static/styles.css`

- [ ] **Step 1: Write the failing UI and dispatch tests**

```javascript
// tests/frontend/tts-provider-ui.test.mjs
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const mainJsSource = readFileSync(resolve(__dirname, "../../static/main-js.js"), "utf8");
const i18nSource = readFileSync(resolve(__dirname, "../../static/js/i18n.js"), "utf8");
const stylesSource = readFileSync(resolve(__dirname, "../../static/styles.css"), "utf8");

test("settings markup includes a provider selector and provider status slot", () => {
  assert.ok(mainJsSource.includes("ttsProviderSelect"));
  assert.ok(mainJsSource.includes("ttsProviderStatus"));
});

test("main-js loads provider metadata from backend helpers", () => {
  assert.ok(mainJsSource.includes("window.FudokiBackendApi.fetchTtsProviders"));
  assert.ok(mainJsSource.includes("window.FudokiBackendApi.requestRemoteSpeech"));
  assert.ok(mainJsSource.includes("window.FudokiRemoteTts.createRemoteTtsPlayer"));
});

test("provider state is stored in localStorage and defaults from backend metadata", () => {
  assert.ok(mainJsSource.includes("ttsProvider: 'ttsProvider'"));
  assert.ok(mainJsSource.includes("default_provider"));
});

test("i18n includes labels for the new provider UI", () => {
  assert.ok(i18nSource.includes("ttsProviderLabel"));
  assert.ok(i18nSource.includes("ttsProviderSystem"));
  assert.ok(i18nSource.includes("ttsProviderRemote"));
  assert.ok(i18nSource.includes("ttsStatusAvailable"));
  assert.ok(i18nSource.includes("ttsStatusRequestFailed"));
});

test("styles include a compact provider status treatment", () => {
  assert.ok(stylesSource.includes(".tts-provider-status"));
  assert.ok(stylesSource.includes(".tts-provider-status.is-error"));
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test tests/frontend/tts-provider-ui.test.mjs`

Expected: FAIL with missing provider UI ids, missing i18n keys, or missing remote player wiring

- [ ] **Step 3: Add provider settings, bootstrap state, and provider-aware playback**

```javascript
// static/main-js.js
const LS = {
  text: 'text',
  voiceURI: 'voiceURI',
  rate: 'rate',
  volume: 'volume',
  texts: 'texts',
  activeId: 'activeId',
  activeFolder: 'activeFolder',
  sortAsc: 'sortAsc',
  twoPane: 'twoPane',
  showKana: 'showKana',
  showRomaji: 'showRomaji',
  showPos: 'showPos',
  showDetails: 'showDetails',
  autoRead: 'autoRead',
  repeatPlay: 'repeatPlay',
  lang: 'lang',
  theme: 'theme',
  lightTheme: 'lightTheme',
  showUnderline: 'showUnderline',
  readingScript: 'readingScript',
  haAsWa: 'haAsWa',
  tokenAlignLeft: 'tokenAlignLeft',
  ttsProvider: 'ttsProvider',
};

let ttsProvider = 'system';
let ttsProvidersPayload = { default_provider: 'system', providers: [{ id: 'system', status: 'available' }] };
let remotePlayer = null;

function createToolbarContentHTML(context) {
  const isSidebar = context === 'sidebar';
  const id = (base) => isSidebar ? `sidebar${base.charAt(0).toUpperCase()}${base.slice(1)}` : base;

  return `
      <div class="settings-section">
        <div class="sidebar-title" id="${id('voiceSettingsTitle')}">${t('voiceTitle')}</div>
        <div class="voice-controls">
          <div class="control-group select-group">
            <label class="control-label" id="${id('ttsProviderLabel')}"><span class="label-text">${t('ttsProviderLabel')}</span></label>
            <select id="${id('ttsProviderSelect')}">
              <option value="system">${t('ttsProviderSystem')}</option>
            </select>
            <div class="tts-provider-status" id="${id('ttsProviderStatus')}">${t('ttsStatusSystemOnly')}</div>
          </div>
          <div class="control-group select-group">
            <label class="control-label" id="${id('voiceSelectLabel')}"><span class="label-text">${t('voiceSelectLabel')}</span></label>
            <select id="${id('voiceSelect')}">
              <option value="">${t('selectVoice')}</option>
            </select>
          </div>
          <div class="control-group full-width">
            <label class="control-label" id="${id('speedLabel')}"><span class="label-text">${t('speedLabel')}</span></label>
            <input type="range" id="${id('speedRange')}" min="0.5" max="2" step="0.1" value="1">
            <div class="speed-display" id="${id('speedValue')}">1.0x</div>
          </div>
        </div>
      </div>
  `;
}

function getVisibleTtsProviderOptions() {
  return (ttsProvidersPayload.providers || []).map((provider) => ({
    id: provider.id,
    label: provider.id === 'openai-compatible' ? t('ttsProviderRemote') : t('ttsProviderSystem'),
    status: provider.status,
    error: provider.error || '',
  }));
}

function syncProviderSelects() {
  const selectIds = ['ttsProviderSelect', 'sidebarTtsProviderSelect'];
  const options = getVisibleTtsProviderOptions();
  selectIds.forEach((selectId) => {
    const select = document.getElementById(selectId);
    if (!select) return;
    select.innerHTML = '';
    options.forEach((provider) => {
      const option = document.createElement('option');
      option.value = provider.id;
      option.textContent = provider.label;
      select.appendChild(option);
    });
    select.value = ttsProvider;
  });
}

function updateTtsProviderStatus() {
  const provider = (ttsProvidersPayload.providers || []).find((entry) => entry.id === ttsProvider);
  const text = !provider
    ? t('ttsStatusSystemOnly')
    : provider.status === 'request_failed'
      ? `${t('ttsStatusRequestFailed')}: ${provider.error || ''}`.trim()
      : provider.id === 'openai-compatible'
        ? t('ttsStatusAvailable')
        : t('ttsStatusSystemOnly');

  ['ttsProviderStatus', 'sidebarTtsProviderStatus'].forEach((statusId) => {
    const el = document.getElementById(statusId);
    if (!el) return;
    el.textContent = text;
    el.classList.toggle('is-error', provider && provider.status === 'request_failed');
  });

  const usingRemote = ttsProvider === 'openai-compatible';
  ['voiceSelect', 'sidebarVoiceSelect', 'headerVoiceSelect'].forEach((id) => {
    const el = document.getElementById(id);
    if (el) el.disabled = usingRemote;
  });
}

async function bootstrapTtsProviders() {
  const backendApi = await waitForBackendApiClient();
  ttsProvidersPayload = await backendApi.fetchTtsProviders();
  const storedProvider = localStorage.getItem(LS.ttsProvider);
  const visibleIds = new Set((ttsProvidersPayload.providers || []).map((entry) => entry.id));
  ttsProvider = visibleIds.has(storedProvider)
    ? storedProvider
    : ttsProvidersPayload.default_provider || 'system';
  localStorage.setItem(LS.ttsProvider, ttsProvider);
  syncProviderSelects();
  updateTtsProviderStatus();
}

function getCurrentTtsProvider() {
  return ttsProvider;
}

function isRemoteTtsSelected() {
  return getCurrentTtsProvider() === 'openai-compatible';
}

async function playTextThroughSelectedProvider(text, mode) {
  if (!text || !text.trim()) return;

  if (!isRemoteTtsSelected() || mode === 'token') {
    speak(text);
    return;
  }

  if (!remotePlayer) {
    remotePlayer = window.FudokiRemoteTts.createRemoteTtsPlayer({
      onStateChange: (state) => {
        if (state === 'error') {
          showNotification(t('ttsStatusRequestFailed'), 'error');
        }
      },
    });
  }

  stopSpeaking();
  const backendApi = await waitForBackendApiClient();
  try {
    const response = await backendApi.requestRemoteSpeech({
      provider: 'openai-compatible',
      text,
      speed: rate,
    });
    await remotePlayer.playResponse(response);
  } catch (error) {
    showNotification(error.message || t('ttsStatusRequestFailed'), 'error');
    const remoteView = (ttsProvidersPayload.providers || []).find((entry) => entry.id === 'openai-compatible');
    if (remoteView) {
      remoteView.status = 'request_failed';
      remoteView.error = error.message || '';
      updateTtsProviderStatus();
    }
  }
}

document.addEventListener('change', (event) => {
  const target = event.target;
  if (!(target instanceof HTMLSelectElement)) return;
  if (target.id !== 'ttsProviderSelect' && target.id !== 'sidebarTtsProviderSelect') return;
  ttsProvider = target.value;
  localStorage.setItem(LS.ttsProvider, ttsProvider);
  syncProviderSelects();
  updateTtsProviderStatus();
});
```

```javascript
// static/js/i18n.js
ttsProviderLabel: '语音提供方',
ttsProviderSystem: '系统语音',
ttsProviderRemote: 'OpenAI 兼容',
ttsStatusSystemOnly: '使用浏览器和系统自带语音',
ttsStatusAvailable: '在线语音可用',
ttsStatusRequestFailed: '在线语音请求失败',
```

```css
/* static/styles.css */
.tts-provider-status {
  margin-top: 6px;
  font-size: 12px;
  line-height: 1.4;
  color: var(--text-secondary, #6b7280);
}

.tts-provider-status.is-error {
  color: #b91c1c;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test tests/frontend/tts-provider-ui.test.mjs`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add static/main-js.js static/js/i18n.js static/styles.css tests/frontend/tts-provider-ui.test.mjs
git commit -m "feat: add tts provider selection and status ui"
```

## Task 6: Route Line and Full-Text Playback Through the Selected Provider and Document `.env`

**Files:**
- Create: `.env.example`
- Modify: `.gitignore`
- Modify: `static/main-js.js`
- Modify: `README.md`

- [ ] **Step 1: Write the failing docs and dispatch tests**

```javascript
// tests/frontend/tts-provider-ui.test.mjs
import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const readmeSource = readFileSync(resolve(__dirname, "../../README.md"), "utf8");
const gitignoreSource = readFileSync(resolve(__dirname, "../../.gitignore"), "utf8");
const envExamplePath = resolve(__dirname, "../../.env.example");
const mainJsSource = readFileSync(resolve(__dirname, "../../static/main-js.js"), "utf8");

test("playback entry points delegate line and full text through the selected provider", () => {
  assert.ok(mainJsSource.includes("playTextThroughSelectedProvider(lineText, 'line')"));
  assert.ok(mainJsSource.includes("playTextThroughSelectedProvider(readingParts, 'full')"));
  assert.ok(mainJsSource.includes("playTextThroughSelectedProvider(text, 'full')"));
  assert.ok(mainJsSource.includes("playTextThroughSelectedProvider(textToSpeak, 'token')"));
});

test(".env.example is present and documents the required remote tts keys", () => {
  assert.equal(existsSync(envExamplePath), true);
  const envExampleSource = readFileSync(envExamplePath, "utf8");
  assert.ok(envExampleSource.includes("FUDOKI_TTS_OPENAI_BASE_URL"));
  assert.ok(envExampleSource.includes("FUDOKI_TTS_OPENAI_API_KEY"));
  assert.ok(envExampleSource.includes("FUDOKI_TTS_OPENAI_MODEL"));
});

test("gitignore excludes the local .env file", () => {
  assert.ok(gitignoreSource.includes(".env"));
});

test("README explains online tts configuration and env defaults", () => {
  assert.ok(readmeSource.includes("FUDOKI_TTS_OPENAI_BASE_URL"));
  assert.ok(readmeSource.includes("FUDOKI_TTS_DEFAULT_PROVIDER"));
  assert.ok(readmeSource.includes("OpenAI-compatible"));
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test tests/frontend/tts-provider-ui.test.mjs`

Expected: FAIL with missing `.env.example`, missing `.env` ignore rule, or missing provider-aware playback calls

- [ ] **Step 3: Finish playback wiring and add configuration docs**

```javascript
// static/main-js.js
window.playLine = function(lineIndex) {
  const lines = Array.from(document.querySelectorAll('.token-line'));
  const line = lines[lineIndex];
  if (!line) return;

  const lineText = Array.from(line.childNodes)
    .map((node) => node.textContent || '')
    .join('')
    .trim();

  playTextThroughSelectedProvider(lineText, 'line');
};

function playAllText() {
  const contentArea = document.getElementById('content');
  if (contentArea) {
    const readingParts = Array.from(contentArea.querySelectorAll('.token-line'))
      .map((node) => node.textContent || '')
      .join('\n')
      .trim();

    if (readingParts) {
      playTextThroughSelectedProvider(readingParts, 'full');
      return;
    }
  }

  const text = textInput.value.trim();
  if (text) {
    playTextThroughSelectedProvider(text, 'full');
  } else {
    showNotification(t('pleaseInputText'), 'warning');
  }
}

function speakTokenNode(node) {
  let textToSpeak = '';
  try {
    const tokenDataAttr = node.getAttribute('data-token') || '{}';
    const tokenData = JSON.parse(tokenDataAttr);
    textToSpeak = resolveTokenSpeechText(tokenData);
  } catch (_error) {
    textToSpeak = node.textContent || '';
  }

  playTextThroughSelectedProvider(textToSpeak, 'token');
}
```

```dotenv
# .env.example
# Show the OpenAI-compatible provider only when all three required values are present.
FUDOKI_TTS_OPENAI_BASE_URL=https://api.openai.com/v1
FUDOKI_TTS_OPENAI_API_KEY=your_api_key_here
FUDOKI_TTS_OPENAI_MODEL=gpt-4o-mini-tts

# Optional defaults used when the user has not chosen a local override yet.
FUDOKI_TTS_OPENAI_VOICE=alloy
FUDOKI_TTS_OPENAI_FORMAT=mp3
FUDOKI_TTS_DEFAULT_PROVIDER=openai-compatible
```

```gitignore
# .gitignore
.DS_Store
.env

# 大文件 - 使用分片版本替代
static/libs/dict/jmdict-eng-3.6.1.json

# Python缓存
__pycache__/
*.pyc
*.pyo

# 临时文件
*.tmp
*.log
target/

# Local git worktrees
.worktrees/
```

```md
<!-- README.md -->
### Online TTS via `.env`

Fudoki can optionally expose an `OpenAI-compatible` online TTS provider in addition to the existing browser system voices.

1. Copy `.env.example` to `.env`
2. Fill in:
   - `FUDOKI_TTS_OPENAI_BASE_URL`
   - `FUDOKI_TTS_OPENAI_API_KEY`
   - `FUDOKI_TTS_OPENAI_MODEL`
3. Optionally set:
   - `FUDOKI_TTS_OPENAI_VOICE`
   - `FUDOKI_TTS_OPENAI_FORMAT`
   - `FUDOKI_TTS_DEFAULT_PROVIDER`

Behavior:

- if the required `.env` keys are absent, Fudoki shows only `System`
- if the required keys exist, Fudoki shows `OpenAI-compatible`
- if the upstream request fails, Fudoki shows a clear error in settings and on playback
- when configured, `.env` defaults are used until the user changes the provider locally
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test tests/frontend/tts-provider-ui.test.mjs`

Expected: PASS

- [ ] **Step 5: Run the full verification suite**

Run: `cargo test && node --test tests/frontend/*.test.mjs`

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add .env.example .gitignore README.md static/main-js.js tests/frontend/tts-provider-ui.test.mjs
git commit -m "feat: document and finish env-driven online tts"
```
