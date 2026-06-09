use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tower::ServiceExt;

struct HiddenDistIndex {
    dist_index: PathBuf,
    hidden_index: PathBuf,
}

impl HiddenDistIndex {
    fn hide(dist_index: impl Into<PathBuf>, hidden_index: impl Into<PathBuf>) -> Self {
        let dist_index = dist_index.into();
        let hidden_index = hidden_index.into();

        if hidden_index.exists() {
            std::fs::remove_file(&hidden_index).unwrap();
        }

        std::fs::rename(&dist_index, &hidden_index).unwrap();
        Self {
            dist_index,
            hidden_index,
        }
    }
}

impl Drop for HiddenDistIndex {
    fn drop(&mut self) {
        if self.hidden_index.exists() {
            std::fs::rename(&self.hidden_index, &self.dist_index).unwrap();
        }
    }
}

fn frontend_dist_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[tokio::test]
async fn root_serves_built_frontend_shell() {
    let _guard = frontend_dist_lock().lock().unwrap();
    let app = fudoki_backend::app::build_router();

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains(r#"<div id="root"></div>"#));
    assert!(html.contains("/assets/"));
}

#[tokio::test]
async fn root_does_not_fall_back_to_legacy_static_shell_when_dist_is_missing() {
    let _guard = frontend_dist_lock().lock().unwrap();
    let dist_index = Path::new("frontend/dist/index.html");
    let hidden_index = Path::new("frontend/dist/index.html.test-hidden");
    let _hidden_dist_index = HiddenDistIndex::hide(dist_index, hidden_index);
    let response = fudoki_backend::app::build_router()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn built_frontend_public_favicon_is_served() {
    let app = fudoki_backend::app::build_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/favicon.svg")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
