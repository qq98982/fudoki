use fudoki_backend::{app::build_router, server::bind_addr_from_env};

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    let bind_addr = bind_addr_from_env();
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("bind listener");
    println!("Fudoki is listening on http://{bind_addr}");

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
