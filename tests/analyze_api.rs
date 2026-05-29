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

#[tokio::test]
async fn analyze_preserves_unknown_english_display_and_tts_text() {
    let app = fudoki_backend::app::build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/analyze")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"text":"browser"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let token = &json["lines"][0][0];

    assert_eq!(token["surface"], "browser");
    assert_eq!(token["reading"], "ブラウザ");
    assert_eq!(token["tts_text"], "ブラウザ");
    assert_eq!(token["source"], "dictionary_corrected");

    // Verify that truly unknown English is still preserved
    let app2 = fudoki_backend::app::build_router();
    let response = app2
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/analyze")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"text":"xyzzy"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let token = &json["lines"][0][0];

    assert_eq!(token["surface"], "xyzzy");
    assert_eq!(token["reading"], "");
    assert_eq!(token["tts_text"], "xyzzy");
    assert_eq!(token["source"], "unknown_english");
}

#[tokio::test]
async fn analyze_normalizes_common_particle_pronunciations() {
    let app = fudoki_backend::app::build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/analyze")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"text":"彼は来る。\n東京へ行く。\n本を読む。"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["lines"][0][1]["surface"], "は");
    assert_eq!(json["lines"][0][1]["reading"], "ワ");
    assert_eq!(json["lines"][0][1]["tts_text"], "ワ");

    assert_eq!(json["lines"][1][1]["surface"], "へ");
    assert_eq!(json["lines"][1][1]["reading"], "エ");
    assert_eq!(json["lines"][1][1]["tts_text"], "エ");

    assert_eq!(json["lines"][2][1]["surface"], "を");
    assert_eq!(json["lines"][2][1]["reading"], "オ");
    assert_eq!(json["lines"][2][1]["tts_text"], "オ");
}
