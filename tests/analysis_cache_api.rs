use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use rusqlite::Connection;
use tower::ServiceExt;

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

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("fudoki-{prefix}-{stamp}"));
    fs::create_dir_all(&path).unwrap();
    path
}

#[tokio::test]
async fn analyze_endpoint_persists_cached_result_and_cached_analysis_endpoint_survives_router_rebuild() {
    let _guard = env_lock().lock().unwrap_or_else(|e| e.into_inner());
    let keys = ["FUDOKI_DATA_DIR"];
    let env = EnvGuard::new(&keys);
    for key in keys {
        env.remove(key);
    }
    let dir = unique_temp_dir("analysis-cache");
    env.set("FUDOKI_DATA_DIR", dir.to_str().unwrap());

    let app = fudoki_backend::app::build_router();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/analyze")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"document_id":"doc-cache-1","document_revision":4,"text":"日本語の文章です"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let analyzed_body = response.into_body().collect().await.unwrap().to_bytes();
    let analyzed: serde_json::Value = serde_json::from_slice(&analyzed_body).unwrap();
    assert!(analyzed["lines"].as_array().unwrap().len() > 0);

    let cached = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/documents/doc-cache-1/analysis?revision=4")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(cached.status(), StatusCode::OK);
    let cached_body = cached.into_body().collect().await.unwrap().to_bytes();
    let cached_json: serde_json::Value = serde_json::from_slice(&cached_body).unwrap();
    assert_eq!(cached_json, analyzed);

    let db = Connection::open(dir.join("fudoki.db")).unwrap();
    let count: i64 = db
        .query_row("SELECT COUNT(*) FROM analysis_cache", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);

    let rebuilt_app = fudoki_backend::app::build_router();
    let cached_again = rebuilt_app
        .oneshot(
            Request::builder()
                .uri("/api/documents/doc-cache-1/analysis?revision=4")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(cached_again.status(), StatusCode::OK);
}

#[tokio::test]
async fn analyze_endpoint_ignores_cached_results_from_older_analyzer_versions() {
    let _guard = env_lock().lock().unwrap_or_else(|e| e.into_inner());
    let keys = ["FUDOKI_DATA_DIR"];
    let env = EnvGuard::new(&keys);
    for key in keys {
        env.remove(key);
    }
    let dir = unique_temp_dir("analysis-cache-version-bump");
    env.set("FUDOKI_DATA_DIR", dir.to_str().unwrap());

    let db = fudoki_backend::storage::db::AppDatabase::new(dir.clone()).unwrap();
    db.with_connection(|conn| {
        conn.execute(
            r#"
            INSERT INTO analysis_cache (
                cache_key,
                document_id,
                document_revision,
                content_hash,
                analyzer_version,
                result_json,
                created_at,
                last_used_at
            ) VALUES (
                'legacy-cache-key',
                'doc-cache-versioned',
                1,
                '6888d2f01f20e5cbd04d792f9201b9ff4f9a59091b2c14b710287d6b3a52e0ba',
                'sudachi-v1',
                '{\"lines\":[[{\"surface\":\"彼\",\"lemma\":\"彼\",\"reading\":\"カレ\",\"tts_text\":\"カレ\",\"pos\":[\"代名詞\"],\"source\":\"sudachi\",\"confidence\":1.0},{\"surface\":\"は\",\"lemma\":\"は\",\"reading\":\"ハ\",\"tts_text\":\"ハ\",\"pos\":[\"助詞\"],\"source\":\"sudachi\",\"confidence\":1.0}]]}',
                0,
                0
            )
            "#,
            [],
        )?;
        Ok(())
    })
    .unwrap();

    let app = fudoki_backend::app::build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/analyze")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"document_id":"doc-cache-versioned","document_revision":1,"text":"彼は"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["lines"][0][1]["surface"], "は");
    assert_eq!(json["lines"][0][1]["reading"], "ワ");
}

#[tokio::test]
async fn clear_analysis_cache_endpoint_removes_only_analysis_cache_entries() {
    let _guard = env_lock().lock().unwrap_or_else(|e| e.into_inner());
    let keys = ["FUDOKI_DATA_DIR"];
    let env = EnvGuard::new(&keys);
    for key in keys {
        env.remove(key);
    }
    let dir = unique_temp_dir("analysis-cache-clear");
    env.set("FUDOKI_DATA_DIR", dir.to_str().unwrap());

    let app = fudoki_backend::app::build_router();

    let analyze_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/analyze")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"document_id":"doc-cache-clear-1","document_revision":1,"text":"彼は"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(analyze_response.status(), StatusCode::OK);

    let update_settings = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/api/settings")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"values":{"theme":"paper","lang":"ja"}}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update_settings.status(), StatusCode::OK);

    let db = Connection::open(dir.join("fudoki.db")).unwrap();
    let before_count: i64 = db
        .query_row("SELECT COUNT(*) FROM analysis_cache", [], |row| row.get(0))
        .unwrap();
    assert_eq!(before_count, 1);

    let clear_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/api/analysis-cache")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(clear_response.status(), StatusCode::NO_CONTENT);

    let after_count: i64 = db
        .query_row("SELECT COUNT(*) FROM analysis_cache", [], |row| row.get(0))
        .unwrap();
    assert_eq!(after_count, 0);

    let settings_value: String = db
        .query_row(
            "SELECT value_json FROM settings WHERE key = 'lang'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(settings_value, "\"ja\"");
}
