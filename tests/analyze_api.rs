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
