pub mod app;

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .expect("bind listener");
    axum::serve(listener, app::build_router())
        .await
        .expect("serve application");
}
