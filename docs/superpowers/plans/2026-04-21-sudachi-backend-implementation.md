# Fudoki Rust Sudachi Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace browser-side tokenization and dictionary lookup with a local Rust backend built on `axum` and `sudachi`, while preserving the current UI and fixing English-term reading behavior.

**Architecture:** Add a Rust backend that serves the existing static frontend and exposes `/api/health`, `/api/analyze`, and `/api/dictionary`. Keep rendering and TTS controls in the frontend, but move token generation, English classification, startup readiness, and dictionary lookup to Rust. Use a dedicated `tts_text` field so playback behavior can diverge safely from display `reading`.

**Tech Stack:** Rust, Cargo, axum, tokio, tower-http, serde, sudachi (git dependency from WorksApplications/sudachi.rs), Node built-in test runner, existing static frontend JS

---

## Planned File Structure

**Create**

- `Cargo.toml`
- `src/main.rs`
- `src/app.rs`
- `src/models.rs`
- `src/english.rs`
- `src/analyzer.rs`
- `src/dictionary.rs`
- `resources/sudachi.json`
- `tests/health.rs`
- `tests/static_serving.rs`
- `tests/english.rs`
- `tests/analyze_api.rs`
- `tests/dictionary_api.rs`
- `static/js/backend-api.js`
- `tests/frontend/backend-api.test.mjs`

**Modify**

- `package.json`
- `README.md`
- `index.html`
- `static/main-js.js`
- `static/js/tts.js`
- `static/pwa-assets.json`

**Keep temporarily, but stop using as primary path**

- `static/segmenter.js`
- `static/libs/dict/dictionary-service.js`
- `static/libs/kuromoji.js`
- `static/libs/kuroshiro.min.js`
- `static/libs/kuroshiro-analyzer-kuromoji.min.js`

## Task 1: Bootstrap Rust Backend and Health Endpoint

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/app.rs`
- Test: `tests/health.rs`

- [ ] **Step 1: Write the failing health-endpoint test**

```rust
// tests/health.rs
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn health_endpoint_reports_service_state() {
    let app = fudoki_backend::app::build_router();

    let response = app
        .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test health`

Expected: FAIL with missing crate or missing module errors

- [ ] **Step 3: Write the minimal Rust backend scaffold**

```toml
# Cargo.toml
[package]
name = "fudoki_backend"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.8.9"
tokio = { version = "1.48.0", features = ["macros", "rt-multi-thread"] }
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.145"
tower = "0.5.2"
tower-http = { version = "0.6.8", features = ["fs"] }

[dev-dependencies]
http-body-util = "0.1.3"
```

```rust
// src/main.rs
pub mod app;

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .expect("bind listener");
    axum::serve(listener, app::build_router())
        .await
        .expect("serve application");
}
```

```rust
// src/app.rs
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test health`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/main.rs src/app.rs tests/health.rs
git commit -m "feat: bootstrap Rust backend health endpoint"
```

## Task 2: Serve Existing Frontend Static Files from Axum

**Files:**
- Modify: `src/app.rs`
- Modify: `package.json`
- Test: `tests/static_serving.rs`

- [ ] **Step 1: Write the failing static-serving tests**

```rust
// tests/static_serving.rs
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn root_serves_index_html() {
    let app = fudoki_backend::app::build_router();

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn static_main_js_is_served() {
    let app = fudoki_backend::app::build_router();

    let response = app
        .oneshot(Request::builder().uri("/static/main-js.js").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test static_serving`

Expected: FAIL with `404 != 200`

- [ ] **Step 3: Mount the repo root and static directory**

```rust
// src/app.rs
use std::path::PathBuf;

use axum::{
    response::IntoResponse,
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
    axum::response::Html(std::fs::read_to_string("index.html").expect("read index.html"))
}

pub fn build_router() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/api/health", get(health))
        .nest_service("/static", ServeDir::new(PathBuf::from("static")))
}
```

```json
// package.json
{
  "name": "fudoki-js",
  "version": "1.0.0",
  "description": "纯JS版本的日语学习工具，支持分词和语音朗读",
  "main": "index.html",
  "scripts": {
    "start": "cargo run --release",
    "dev": "cargo run",
    "serve": "cargo run --release",
    "test": "cargo test && node --test tests/frontend/*.test.mjs"
  },
  "dependencies": {
    "jmdict-simplified-node": "^1.1.2",
    "kuromoji": "^0.1.2",
    "kuroshiro": "^1.2.0",
    "kuroshiro-analyzer-kuromoji": "^1.1.0"
  },
  "devDependencies": {
    "vite": "^5.0.0"
  },
  "keywords": ["japanese", "morphological-analysis", "tts", "learning"],
  "author": "Cheyan",
  "license": "MIT"
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test static_serving`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/app.rs package.json tests/static_serving.rs
git commit -m "feat: serve frontend assets from axum"
```

## Task 3: Add English Classification Rules with TDD

**Files:**
- Create: `src/english.rs`
- Test: `tests/english.rs`

- [ ] **Step 1: Write the failing English-classifier tests**

```rust
// tests/english.rs
use fudoki_backend::english::classify_english_token;

#[test]
fn all_caps_token_is_treated_as_abbreviation() {
    let result = classify_english_token("API");
    assert_eq!(result.kind, "abbreviation");
    assert_eq!(result.reading, "エーピーアイ");
    assert_eq!(result.tts_text, "エーピーアイ");
}

#[test]
fn known_technical_term_uses_japanese_usage() {
    let result = classify_english_token("React");
    assert_eq!(result.kind, "override");
    assert_eq!(result.reading, "リアクト");
    assert_eq!(result.tts_text, "リアクト");
}

#[test]
fn unknown_english_term_keeps_original_text_for_tts() {
    let result = classify_english_token("browser");
    assert_eq!(result.kind, "unknown");
    assert_eq!(result.reading, "");
    assert_eq!(result.tts_text, "browser");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test english`

Expected: FAIL with missing module or missing function errors

- [ ] **Step 3: Implement the minimal classifier**

```rust
// src/english.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnglishToken {
    pub kind: &'static str,
    pub reading: String,
    pub tts_text: String,
    pub source: &'static str,
    pub confidence: u8,
}

fn abbreviation_reading(token: &str) -> String {
    token
        .chars()
        .map(|ch| match ch {
            'A' => "エー",
            'B' => "ビー",
            'C' => "シー",
            'D' => "ディー",
            'E' => "イー",
            'F' => "エフ",
            'G' => "ジー",
            'H' => "エイチ",
            'I' => "アイ",
            'J' => "ジェー",
            'K' => "ケー",
            'L' => "エル",
            'M' => "エム",
            'N' => "エヌ",
            'O' => "オー",
            'P' => "ピー",
            'Q' => "キュー",
            'R' => "アール",
            'S' => "エス",
            'T' => "ティー",
            'U' => "ユー",
            'V' => "ブイ",
            'W' => "ダブリュー",
            'X' => "エックス",
            'Y' => "ワイ",
            'Z' => "ズィー",
            _ => "",
        })
        .collect()
}

pub fn classify_english_token(token: &str) -> EnglishToken {
    let lower = token.to_ascii_lowercase();
    let override_reading = match lower.as_str() {
        "react" => Some("リアクト"),
        "docker" => Some("ドッカー"),
        "github" => Some("ギットハブ"),
        "typescript" => Some("タイプスクリプト"),
        "javascript" => Some("ジャバスクリプト"),
        _ => None,
    };

    if token.chars().all(|c| c.is_ascii_uppercase()) {
        let reading = abbreviation_reading(token);
        return EnglishToken {
            kind: "abbreviation",
            reading: reading.clone(),
            tts_text: reading,
            source: "abbreviation",
            confidence: 100,
        };
    }

    if let Some(reading) = override_reading {
        return EnglishToken {
            kind: "override",
            reading: reading.to_string(),
            tts_text: reading.to_string(),
            source: "override",
            confidence: 100,
        };
    }

    EnglishToken {
        kind: "unknown",
        reading: String::new(),
        tts_text: token.to_string(),
        source: "unknown_english",
        confidence: 0,
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test english`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/english.rs tests/english.rs
git commit -m "feat: add English token classification rules in Rust"
```

## Task 4: Add `sudachi` Analyzer and `/api/analyze`

**Files:**
- Create: `src/models.rs`
- Create: `src/analyzer.rs`
- Modify: `Cargo.toml`
- Modify: `src/main.rs`
- Modify: `src/app.rs`
- Create: `resources/sudachi.json`
- Test: `tests/analyze_api.rs`

- [ ] **Step 1: Write the failing analyze-endpoint tests**

```rust
// tests/analyze_api.rs
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn analyze_returns_required_token_fields() {
    let app = fudoki_backend::app::build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/analyze")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"text":"React API browser"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let token = &json["lines"][0][0];
    assert!(token.get("surface").is_some());
    assert!(token.get("lemma").is_some());
    assert!(token.get("reading").is_some());
    assert!(token.get("tts_text").is_some());
    assert!(token.get("pos").is_some());
    assert!(token.get("source").is_some());
    assert!(token.get("confidence").is_some());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test analyze_api`

Expected: FAIL with `404 != 200`

- [ ] **Step 3: Add the `sudachi` dependency and analyzer implementation**

```toml
# Cargo.toml
[dependencies]
axum = "0.8.9"
tokio = { version = "1.48.0", features = ["macros", "rt-multi-thread"] }
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.145"
tower = "0.5.2"
tower-http = { version = "0.6.8", features = ["fs"] }
sudachi = { git = "https://github.com/WorksApplications/sudachi.rs.git", package = "sudachi" }
```

```json
// resources/sudachi.json
{
  "path": "resources/",
  "systemDict": "system.dic"
}
```

```rust
// src/models.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub text: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct TokenPayload {
    pub surface: String,
    pub lemma: String,
    pub reading: String,
    pub tts_text: String,
    pub pos: Vec<String>,
    pub source: String,
    pub confidence: f32,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeResponse {
    pub lines: Vec<Vec<TokenPayload>>,
}
```

```rust
// src/analyzer.rs
use std::path::PathBuf;
use std::sync::Arc;

use sudachi::analysis::stateless_tokenizer::StatelessTokenizer;
use sudachi::analysis::Mode;
use sudachi::analysis::Tokenize;
use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;

use crate::english::classify_english_token;
use crate::models::TokenPayload;

pub struct Analyzer {
    tokenizer: StatelessTokenizer<Arc<JapaneseDictionary>>,
}

impl Analyzer {
    pub fn new() -> Self {
        let config = Config::new(Some(PathBuf::from("resources/sudachi.json")), None, None)
            .expect("load sudachi config");
        let dictionary = JapaneseDictionary::from_cfg(&config).expect("load sudachi dictionary");
        let tokenizer = StatelessTokenizer::new(Arc::new(dictionary));
        Self { tokenizer }
    }

    pub fn analyze(&self, text: &str) -> Vec<Vec<TokenPayload>> {
        text.lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }
                let morphemes = self.tokenizer.tokenize(line, Mode::C, false).expect("tokenize");
                let tokens = morphemes
                    .iter()
                    .map(|morpheme| {
                        let surface = morpheme.surface().to_string();
                        if surface.chars().all(|c| c.is_ascii_alphanumeric() || ".+-_".contains(c)) {
                            let english = classify_english_token(&surface);
                            return TokenPayload {
                                surface: surface.clone(),
                                lemma: surface,
                                reading: english.reading,
                                tts_text: english.tts_text,
                                pos: vec!["名詞".to_string()],
                                source: english.source.to_string(),
                                confidence: english.confidence as f32 / 100.0,
                            };
                        }
                        TokenPayload {
                            surface,
                            lemma: morpheme.dictionary_form().to_string(),
                            reading: morpheme.reading_form().to_string(),
                            tts_text: {
                                let reading = morpheme.reading_form();
                                if reading.is_empty() {
                                    morpheme.surface().to_string()
                                } else {
                                    reading.to_string()
                                }
                            },
                            pos: morpheme.part_of_speech().to_vec(),
                            source: "sudachi".to_string(),
                            confidence: 1.0,
                        }
                    })
                    .collect::<Vec<_>>();
                Some(tokens)
            })
            .collect()
    }
}
```

```rust
// src/app.rs
use std::sync::Arc;

use axum::{
    extract::State,
    response::IntoResponse,
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
    axum::response::Html(std::fs::read_to_string("index.html").expect("read index.html"))
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
```

```rust
// src/main.rs
pub mod analyzer;
pub mod app;
pub mod english;
pub mod models;

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .expect("bind listener");
    axum::serve(listener, app::build_router())
        .await
        .expect("serve application");
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test analyze_api`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/main.rs src/app.rs src/models.rs src/analyzer.rs resources/sudachi.json tests/analyze_api.rs
git commit -m "feat: add Sudachi analyze endpoint in Rust"
```

## Task 5: Add Indexed JMdict Lookup and `/api/dictionary`

**Files:**
- Create: `src/dictionary.rs`
- Modify: `src/main.rs`
- Modify: `src/app.rs`
- Test: `tests/dictionary_api.rs`

- [ ] **Step 1: Write the failing dictionary-endpoint test**

```rust
// tests/dictionary_api.rs
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn dictionary_response_matches_frontend_shape() {
    let app = fudoki_backend::app::build_router();

    let response = app
        .oneshot(Request::builder().uri("/api/dictionary?term=%E3%82%A6%E3%82%A7%E3%83%96").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("word").is_some());
    assert!(json.get("query").is_some());
    assert!(json.get("kanji").is_some());
    assert!(json.get("kana").is_some());
    assert!(json.get("senses").is_some());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test dictionary_api`

Expected: FAIL with `404 != 200`

- [ ] **Step 3: Implement indexed lookup over current chunked JMdict data**

```rust
// src/dictionary.rs
use std::collections::HashMap;
use std::fs;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
struct ChunkRoot {
    words: Vec<Entry>,
}

#[derive(Debug, Clone, Deserialize)]
struct Entry {
    kanji: Option<Vec<TextValue>>,
    kana: Option<Vec<TextValue>>,
    sense: Option<Vec<Sense>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextValue {
    pub text: String,
    #[serde(default)]
    pub common: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct Gloss {
    text: String,
}

#[derive(Debug, Clone, Deserialize)]
struct LanguageSource {
    lang: String,
    text: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Sense {
    #[serde(default)]
    partOfSpeech: Vec<String>,
    #[serde(default)]
    field: Vec<String>,
    #[serde(default)]
    misc: Vec<String>,
    #[serde(default)]
    info: Vec<String>,
    #[serde(default)]
    gloss: Vec<Gloss>,
    #[serde(default)]
    languageSource: Vec<LanguageSource>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SensePayload {
    pub gloss: String,
    pub partOfSpeech: Vec<String>,
    pub field: Vec<String>,
    pub misc: Vec<String>,
    pub info: Vec<String>,
    pub chineseSource: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DictionaryPayload {
    pub word: String,
    pub query: String,
    pub kanji: Vec<TextValue>,
    pub kana: Vec<TextValue>,
    pub senses: Vec<SensePayload>,
    pub hasMultipleMeanings: bool,
    pub totalResults: usize,
    pub lookupSource: String,
}

#[derive(Default)]
pub struct DictionaryService {
    index: HashMap<String, Vec<Entry>>,
}

impl DictionaryService {
    pub fn new() -> Self {
        let mut service = Self::default();
        for path in fs::read_dir("static/libs/dict/chunks").expect("read chunk dir") {
            let path = path.expect("dir entry").path();
            if !path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("jmdict_chunk_"))
            {
                continue;
            }
            let text = fs::read_to_string(path).expect("read chunk file");
            let chunk: ChunkRoot = serde_json::from_str(&text).expect("parse chunk");
            for entry in chunk.words {
                if let Some(kanji) = &entry.kanji {
                    for item in kanji {
                        service.index.entry(item.text.clone()).or_default().push(entry.clone());
                    }
                }
                if let Some(kana) = &entry.kana {
                    for item in kana {
                        service.index.entry(item.text.clone()).or_default().push(entry.clone());
                    }
                }
            }
        }
        service
    }

    pub fn lookup(&self, term: &str) -> Option<DictionaryPayload> {
        let entries = self.index.get(term)?;
        let entry = entries.first()?.clone();
        let senses = entry
            .sense
            .unwrap_or_default()
            .into_iter()
            .map(|sense| SensePayload {
                gloss: sense.gloss.into_iter().map(|g| g.text).collect::<Vec<_>>().join("; "),
                partOfSpeech: sense.partOfSpeech,
                field: sense.field,
                misc: sense.misc,
                info: sense.info,
                chineseSource: sense
                    .languageSource
                    .into_iter()
                    .find(|ls| ls.lang == "chi")
                    .map(|ls| ls.text),
            })
            .collect::<Vec<_>>();

        Some(DictionaryPayload {
            word: term.to_string(),
            query: term.to_string(),
            kanji: entry.kanji.unwrap_or_default(),
            kana: entry.kana.unwrap_or_default(),
            hasMultipleMeanings: senses.len() > 1,
            totalResults: entries.len(),
            senses,
            lookupSource: "jmdict".to_string(),
        })
    }
}
```

```rust
// src/app.rs
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;

use crate::analyzer::Analyzer;
use crate::dictionary::DictionaryService;
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

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ready",
        tokenizer_ready: true,
        dictionary_ready: true,
    })
}

async fn index() -> impl IntoResponse {
    axum::response::Html(std::fs::read_to_string("index.html").expect("read index.html"))
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
) -> Result<Json<crate::dictionary::DictionaryPayload>, StatusCode> {
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
```

```rust
// src/main.rs
pub mod analyzer;
pub mod app;
pub mod dictionary;
pub mod english;
pub mod models;

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .expect("bind listener");
    axum::serve(listener, app::build_router())
        .await
        .expect("serve application");
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test dictionary_api`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/dictionary.rs src/app.rs src/main.rs tests/dictionary_api.rs
git commit -m "feat: add indexed JMdict dictionary endpoint in Rust"
```

## Task 6: Add Frontend Backend Client with Readiness Polling

**Files:**
- Create: `static/js/backend-api.js`
- Create: `tests/frontend/backend-api.test.mjs`
- Modify: `index.html`

- [ ] **Step 1: Write the failing frontend helper tests**

```javascript
// tests/frontend/backend-api.test.mjs
import test from "node:test";
import assert from "node:assert/strict";

import { resolveTtsText, waitForBackendReady } from "../../static/js/backend-api.js";

test("resolveTtsText prefers tts_text over reading", () => {
  assert.equal(resolveTtsText({ reading: "ブラウザー", tts_text: "browser" }), "browser");
});

test("waitForBackendReady retries until health endpoint returns ready", async () => {
  let calls = 0;
  const fetcher = async () => {
    calls += 1;
    if (calls < 3) {
      return { ok: true, json: async () => ({ status: "starting" }) };
    }
    return {
      ok: true,
      json: async () => ({ status: "ready", tokenizer_ready: true, dictionary_ready: true }),
    };
  };

  const result = await waitForBackendReady({ fetcher, retries: 3, delayMs: 0 });
  assert.equal(result.status, "ready");
  assert.equal(calls, 3);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test tests/frontend/backend-api.test.mjs`

Expected: FAIL with missing module or export errors

- [ ] **Step 3: Implement the backend client helper**

```javascript
// static/js/backend-api.js
export function resolveTtsText(token) {
  if (token && typeof token.tts_text === "string" && token.tts_text) return token.tts_text;
  if (token && typeof token.reading === "string" && token.reading) return token.reading;
  return token && token.surface ? token.surface : "";
}

export async function waitForBackendReady({
  fetcher = fetch,
  retries = 6,
  delayMs = 500,
} = {}) {
  let last = { status: "starting" };
  for (let attempt = 0; attempt < retries; attempt += 1) {
    const response = await fetcher("/api/health");
    if (response.ok) {
      last = await response.json();
      if (last.status === "ready") return last;
    }
    if (attempt < retries - 1) {
      await new Promise((resolve) => setTimeout(resolve, delayMs * (attempt + 1)));
    }
  }
  return last;
}

export async function analyzeTextRequest(text, fetcher = fetch) {
  const response = await fetcher("/api/analyze", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ text }),
  });
  if (!response.ok) throw new Error(`analyze failed: ${response.status}`);
  return response.json();
}

export async function lookupDictionaryRequest(term, fetcher = fetch) {
  const response = await fetcher(`/api/dictionary?term=${encodeURIComponent(term)}`);
  if (response.status === 404) return null;
  if (!response.ok) throw new Error(`dictionary failed: ${response.status}`);
  return response.json();
}

if (typeof window !== "undefined") {
  window.FudokiBackendApi = {
    resolveTtsText,
    waitForBackendReady,
    analyzeTextRequest,
    lookupDictionaryRequest,
  };
}
```

```html
<!-- index.html -->
<script type="module" src="static/js/backend-api.js"></script>
<script src="static/js/tts.js"></script>
<script src="static/js/i18n.js"></script>
<script src="static/main-js.js"></script>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test tests/frontend/backend-api.test.mjs`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add static/js/backend-api.js tests/frontend/backend-api.test.mjs index.html
git commit -m "feat: add frontend backend API helper"
```

## Task 7: Integrate Frontend Analysis, TTS, and Dictionary with Backend

**Files:**
- Modify: `static/main-js.js`
- Modify: `static/js/tts.js`

- [ ] **Step 1: Write the failing frontend playback tests**

```javascript
// tests/frontend/backend-api.test.mjs
import test from "node:test";
import assert from "node:assert/strict";

import { resolveTtsText } from "../../static/js/backend-api.js";

test("resolveTtsText falls back to reading when tts_text is missing", () => {
  assert.equal(resolveTtsText({ reading: "リアクト", surface: "React" }), "リアクト");
});

test("resolveTtsText falls back to surface when reading and tts_text are empty", () => {
  assert.equal(resolveTtsText({ reading: "", tts_text: "", surface: "browser" }), "browser");
});
```

- [ ] **Step 2: Run tests to verify baseline still passes before integration**

Run: `npm test`

Expected: PASS

- [ ] **Step 3: Switch frontend analysis and lookup to backend helper**

```javascript
// static/main-js.js
async function analyzeText() {
  const raw = textInput.value.trim();
  if (!raw) {
    showEmptyState();
    return;
  }

  showLoadingState();

  try {
    const result = await window.FudokiBackendApi.analyzeTextRequest(filterParentheses(raw));
    displayResults(result);
  } catch (error) {
    console.error("分析错误:", error);
    showErrorState(error.message);
  }
}

async function loadTranslation(element) {
  const tokenData = JSON.parse(element.getAttribute("data-token"));
  let translationContent = element.querySelector(".translation-content");
  if (!translationContent && activeTokenDetails?.element === element) {
    translationContent = activeTokenDetails.details.querySelector(".translation-content");
  }
  if (!translationContent) return;

  try {
    const override = window.FudokiDict?.getTechOverride?.(tokenData);
    if (override?.translations) {
      const lang = typeof currentLang === "string" ? currentLang : "ja";
      const text = override.translations[lang] || override.translations.ja || "";
      if (text) {
        translationContent.textContent = text;
        return;
      }
    }

    const query = tokenData.lemma || tokenData.reading || tokenData.surface;
    const detailedInfo = await window.FudokiBackendApi.lookupDictionaryRequest(query);
    if (!detailedInfo) {
      translationContent.textContent = t("no_translation") || "未找到翻译";
      return;
    }

    const mainTranslation = detailedInfo.senses[0]?.gloss || "";
    translationContent.innerHTML = `<span class="main-translation">${mainTranslation}</span>`;
  } catch (error) {
    translationContent.textContent = t("translation_failed") || "翻译加载失败";
  }
}

async function bootstrapAnalysis() {
  const health = await window.FudokiBackendApi.waitForBackendReady();
  if (health.status !== "ready") {
    showErrorState("backend not ready");
    return;
  }
  if (textInput.value.trim()) {
    await analyzeText();
  } else {
    showEmptyState();
  }
}
```

```javascript
// static/js/tts.js
window.playToken = function playToken(text, event, tokenData) {
  if (event) event.stopPropagation();
  if (window.isPlaying) window.stopSpeaking();

  let resolvedToken = tokenData;
  if (!resolvedToken && event?.target?.closest) {
    const pill = event.target.closest(".token-pill");
    if (pill) {
      const raw = pill.getAttribute("data-token");
      if (raw) resolvedToken = JSON.parse(raw.replace(/&apos;/g, "'"));
    }
  }

  const textToSpeak = window.FudokiBackendApi.resolveTtsText(resolvedToken || { surface: text });
  const pillElement = event?.target?.closest ? event.target.closest(".token-pill") : null;
  const highlightText = resolvedToken?.surface || text;
  window.highlightToken(highlightText, pillElement);
  window.speak(textToSpeak);
};
```

- [ ] **Step 4: Replace the old startup entrypoint**

```javascript
// static/main-js.js
window.analyzeText = analyzeText;
bootstrapAnalysis();
```

- [ ] **Step 5: Run integration tests**

Run: `npm test && cargo test --test analyze_api --test dictionary_api`

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add static/main-js.js static/js/tts.js tests/frontend/backend-api.test.mjs
git commit -m "feat: connect frontend to Rust backend APIs"
```

## Task 8: Remove Browser Tokenizer from Primary Runtime and Document Runbook

**Files:**
- Modify: `index.html`
- Modify: `static/pwa-assets.json`
- Modify: `README.md`

- [ ] **Step 1: Write the failing documentation check**

Run:

```bash
rg -n "cargo run|resources/system.dic|/api/analyze|/api/dictionary" README.md
```

Expected: no matches

- [ ] **Step 2: Remove old tokenizer scripts from primary runtime path**

```html
<!-- index.html -->
<script src="static/js/ui-utils.js"></script>
<script type="module" src="static/js/backend-api.js"></script>
<script src="static/js/tts.js"></script>
<script src="static/js/i18n.js"></script>
<script src="static/main-js.js"></script>
```

```json
// static/pwa-assets.json
{
  "version": "1.1.0",
  "assets": [
    "index.html",
    "static/styles.css",
    "static/main-js.js",
    "static/js/backend-api.js",
    "static/js/tts.js",
    "static/js/ui-utils.js",
    "static/js/i18n.js",
    "static/favicon.svg",
    "static/logo.png",
    "static/logo.svg",
    "static/flags/ja.svg",
    "static/flags/en.svg",
    "static/flags/zh.svg"
  ]
}
```

- [ ] **Step 3: Add the local Rust runbook**

```md
# README.md

## Local Development

Place a Sudachi dictionary file at `resources/system.dic`.

```bash
npm install
cargo run
```

Then open `http://127.0.0.1:8000`.

### Backend APIs

- `GET /api/health`
- `POST /api/analyze`
- `GET /api/dictionary?term=...`
```

- [ ] **Step 4: Run end-to-end verification**

Run:

```bash
npm test
cargo test
timeout 10 cargo run
```

Expected:

- all tests PASS
- Rust server starts on `http://127.0.0.1:8000`

- [ ] **Step 5: Commit**

```bash
git add index.html static/pwa-assets.json README.md
git commit -m "docs: add Rust backend runbook and trim legacy runtime assets"
```

## Spec Coverage Check

- Rust local backend serving static frontend: Task 1, Task 2
- `sudachi.rs` tokenizer path: Task 4
- disk-based Sudachi dictionary runtime: Task 4, Task 8
- English token handling, abbreviation vs unknown: Task 3, Task 4
- `tts_text` contract: Task 3, Task 4, Task 6, Task 7
- Indexed local dictionary lookup: Task 5
- Frontend migration to backend APIs: Task 6, Task 7
- Startup health gating: Task 6, Task 7
- Remove silent browser fallback: Task 8
- Local development instructions: Task 8

## Placeholder Scan

- No `TBD`
- No `TODO`
- No “implement later”
- Every task includes file paths, test commands, and commit commands

## Type Consistency Check

- Analyze token contract uses `surface`, `lemma`, `reading`, `tts_text`, `pos`, `source`, `confidence` consistently across Rust backend and frontend tasks.
- Dictionary contract uses `word`, `query`, `kanji`, `kana`, `senses`, `hasMultipleMeanings`, `totalResults`, `lookupSource` consistently across backend and frontend tasks.
