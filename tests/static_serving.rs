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
        .oneshot(
            Request::builder()
                .uri("/static/main-js.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn login_page_is_served() {
    let app = fudoki_backend::app::build_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/login.html")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
