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
async fn settings_round_trip_persists_theme_language_and_tts_preferences() {
    let _guard = env_lock().lock().unwrap_or_else(|e| e.into_inner());
    let keys = ["FUDOKI_DATA_DIR"];
    let env = EnvGuard::new(&keys);
    for key in keys {
        env.remove(key);
    }
    let dir = unique_temp_dir("settings");
    env.set("FUDOKI_DATA_DIR", dir.to_str().unwrap());

    let app = fudoki_backend::app::build_router();

    let update = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/api/settings")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"values":{"theme":"paper","lang":"ja","ttsProvider":"system","showKana":true}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["values"]["theme"], "paper");
    assert_eq!(json["values"]["lang"], "ja");
    assert_eq!(json["values"]["ttsProvider"], "system");
    assert_eq!(json["values"]["showKana"], true);
}
