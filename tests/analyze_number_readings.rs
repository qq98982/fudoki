use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn analyze_adds_readings_for_common_numeric_tokens() {
    let app = fudoki_backend::app::build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/analyze")
                .header("content-type", "application/json")
                .body(Body::from("{\"text\":\"第1条\\n3人\\n1つ\\n2024年5月7日\"}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["lines"][0][1]["surface"], "1");
    assert_eq!(json["lines"][0][1]["reading"], "イチ");
    assert_eq!(json["lines"][0][1]["tts_text"], "イチ");

    assert_eq!(json["lines"][1][0]["surface"], "3");
    assert_eq!(json["lines"][1][0]["reading"], "サン");
    assert_eq!(json["lines"][1][0]["tts_text"], "サン");

    assert_eq!(json["lines"][2][0]["surface"], "1");
    assert_eq!(json["lines"][2][0]["reading"], "ヒト");
    assert_eq!(json["lines"][2][0]["tts_text"], "ヒト");

    assert_eq!(json["lines"][3][0]["surface"], "2");
    assert_eq!(json["lines"][3][0]["reading"], "ニ");
    assert_eq!(json["lines"][3][0]["tts_text"], "ニ");
    assert_eq!(json["lines"][3][1]["surface"], "0");
    assert_eq!(json["lines"][3][1]["reading"], "ゼロ");
    assert_eq!(json["lines"][3][1]["tts_text"], "ゼロ");
    assert_eq!(json["lines"][3][2]["surface"], "2");
    assert_eq!(json["lines"][3][2]["reading"], "ニ");
    assert_eq!(json["lines"][3][2]["tts_text"], "ニ");
    assert_eq!(json["lines"][3][3]["surface"], "4");
    assert_eq!(json["lines"][3][3]["reading"], "ヨン");
    assert_eq!(json["lines"][3][3]["tts_text"], "ヨン");
    assert_eq!(json["lines"][3][5]["surface"], "5");
    assert_eq!(json["lines"][3][5]["reading"], "ゴ");
    assert_eq!(json["lines"][3][5]["tts_text"], "ゴ");
    assert_eq!(json["lines"][3][7]["surface"], "7");
    assert_eq!(json["lines"][3][7]["reading"], "ナナ");
    assert_eq!(json["lines"][3][7]["tts_text"], "ナナ");
}
