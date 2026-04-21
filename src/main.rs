use fudoki_backend::app::build_router;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .expect("bind listener");
    axum::serve(listener, build_router())
        .await
        .expect("serve application");
}
