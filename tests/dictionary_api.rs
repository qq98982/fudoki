use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn dictionary_response_matches_frontend_shape() {
    let app = fudoki_backend::app::build_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/dictionary?term=%E3%82%A6%E3%82%A7%E3%83%96")
                .body(Body::empty())
                .unwrap(),
        )
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
    assert!(json.get("hasMultipleMeanings").is_some());
    assert!(json.get("totalResults").is_some());
    assert!(json.get("lookupSource").is_some());

    let senses = json.get("senses").and_then(|v| v.as_array()).unwrap();
    assert!(!senses.is_empty());
    let first_sense = senses.first().unwrap();
    assert!(first_sense.get("gloss").is_some());
    assert!(first_sense.get("partOfSpeech").is_some());
    assert!(first_sense.get("field").is_some());
    assert!(first_sense.get("misc").is_some());
    assert!(first_sense.get("info").is_some());
    assert!(first_sense.get("chineseSource").is_some());
}

#[tokio::test]
async fn health_reports_dictionary_ready_after_wiring_dictionary_service() {
    let app = fudoki_backend::app::build_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["dictionary_ready"], true);
}
