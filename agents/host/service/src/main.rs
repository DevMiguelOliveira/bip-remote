use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::accept::Accept;
use tracing::{info, error, warn, debug};
use tracing_subscriber::FmtSubscriber;

// Authenticator types
mod authenticator;
mod session_manager;
mod ipc_handler;
mod hole_punching;

#[tokio::main]
async fn main() {
    // Initialize tracing subscriber
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    info!("bipremote-host-service starting");

    // Listen for IPC connections from the admin console
    let addr: SocketAddr = "127.0.0.1:8081".parse().expect("Invalid address");
    let listener = TcpListener::bind(addr).await
        .expect("Failed to bind to address");

    info!("bipremote-host-service listening on {}", addr);

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("Accepted IPC connection from {}", addr);
                // Spawn a task to handle the connection
                tokio::spawn(handle_ipc_connection(stream));
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

/// Handle an IPC connection from the admin console
async fn handle_ipc_connection(stream: TcpStream) {
    let addr = stream.local_addr().unwrap_or_default();

    // Convert TcpStream to hyper service for HTTP IPC
    let make_svc = hyper::service::service_fn(move |req: hyper::Request<hyper::body::Bytes>| {
        let stream = stream.clone();
        async move { ipc_handler::handle_request(req, stream).await }
    });

    // Run a simple HTTP server over the TCP stream for IPC
    let _ = hyper::Server::builder()
        .http1_title_case(true)
        .serve_connection(stream)
        .await;
}