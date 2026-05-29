use fudoki_backend::app::build_router;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .expect("bind listener");

    let server = axum::serve(listener, build_router());

    tokio::select! {
        result = server => {
            result.expect("serve application");
        }
        _ = shutdown_signal() => {
            eprintln!("shutdown: received signal, stopping gracefully");
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install ctrl+c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    eprintln!();
}
