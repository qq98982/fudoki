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
async fn documents_endpoint_lists_created_documents_and_tracks_active_document() {
    let _guard = env_lock().lock().unwrap_or_else(|e| e.into_inner());
    let keys = ["FUDOKI_DATA_DIR"];
    let env = EnvGuard::new(&keys);
    for key in keys {
        env.remove(key);
    }
    let dir = unique_temp_dir("documents-list");
    env.set("FUDOKI_DATA_DIR", dir.to_str().unwrap());

    let app = fudoki_backend::app::build_router();

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/documents")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"content":"最初の文章\nReact rewrite","title_mode":"auto"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create.status(), StatusCode::CREATED);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/documents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let documents = json["documents"].as_array().unwrap();
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0]["title"], "最初の文章");
    assert_eq!(documents[0]["revision"], 1);
    assert_eq!(json["active_document_id"], documents[0]["id"]);
}

#[tokio::test]
async fn documents_endpoint_updates_document_with_expected_revision() {
    let _guard = env_lock().lock().unwrap_or_else(|e| e.into_inner());
    let keys = ["FUDOKI_DATA_DIR"];
    let env = EnvGuard::new(&keys);
    for key in keys {
        env.remove(key);
    }
    let dir = unique_temp_dir("documents-update");
    env.set("FUDOKI_DATA_DIR", dir.to_str().unwrap());

    let app = fudoki_backend::app::build_router();

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/documents")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"content":"旧い内容","title_mode":"auto"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create.status(), StatusCode::CREATED);
    let create_body = create.into_body().collect().await.unwrap().to_bytes();
    let created: serde_json::Value = serde_json::from_slice(&create_body).unwrap();
    let id = created["document"]["id"].as_str().unwrap();

    let update = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/documents/{id}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"手動タイトル","title_mode":"custom","content":"新しい内容","expected_revision":1}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update.status(), StatusCode::OK);
    let update_body = update.into_body().collect().await.unwrap().to_bytes();
    let updated: serde_json::Value = serde_json::from_slice(&update_body).unwrap();
    assert_eq!(updated["document"]["title"], "手動タイトル");
    assert_eq!(updated["document"]["revision"], 2);

    let stale = app
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/documents/{id}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"title":"古い更新","title_mode":"custom","content":"衝突","expected_revision":1}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(stale.status(), StatusCode::CONFLICT);
}
