use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
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
async fn migration_endpoint_imports_legacy_documents_settings_and_active_document() {
    let _guard = env_lock().lock().unwrap_or_else(|e| e.into_inner());
    let keys = ["FUDOKI_DATA_DIR"];
    let env = EnvGuard::new(&keys);
    for key in keys {
        env.remove(key);
    }
    let dir = unique_temp_dir("legacy-import");
    env.set("FUDOKI_DATA_DIR", dir.to_str().unwrap());

    let app = fudoki_backend::app::build_router();

    let migrate = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/migrations/legacy-browser-data")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{
                      "documents":[
                        {"id":"doc-old-1","content":["第一行","第二行"],"createdAt":1700000000000,"updatedAt":1700000001000},
                        {"id":"doc-old-2","content":"第二个文档","createdAt":1700000002000,"updatedAt":1700000003000}
                      ],
                      "activeId":"doc-old-2",
                      "settings":{
                        "theme":"paper",
                        "lang":"zh",
                        "showKana":true,
                        "ttsProvider":"system"
                      }
                    }"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(migrate.status(), StatusCode::OK);

    let documents = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/documents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(documents.status(), StatusCode::OK);
    let docs_body = documents.into_body().collect().await.unwrap().to_bytes();
    let docs_json: serde_json::Value = serde_json::from_slice(&docs_body).unwrap();
    let docs = docs_json["documents"].as_array().unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs_json["active_document_id"], "doc-old-2");
    assert_eq!(docs[0]["revision"], 1);

    let settings = app
        .oneshot(
            Request::builder()
                .uri("/api/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(settings.status(), StatusCode::OK);
    let settings_body = settings.into_body().collect().await.unwrap().to_bytes();
    let settings_json: serde_json::Value = serde_json::from_slice(&settings_body).unwrap();
    assert_eq!(settings_json["values"]["theme"], "paper");
    assert_eq!(settings_json["values"]["lang"], "zh");
    assert_eq!(settings_json["values"]["showKana"], true);
}
