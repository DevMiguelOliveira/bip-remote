use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::accept::Accept;
use tracing::{info, error, warn};
use tracing_subscriber::FmtSubscriber;

// Authenticator types
mod authenticator;
mod session_manager;
mod ipc_handler;

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
    use hyper::{Body, Server, service::service_fn};
    use hyper::http::Request;
    use hyper::body::Bytes;

    let addr = stream.local_addr().unwrap_or_default();

    // Convert TcpStream to hyper service
    let make_svc = service_fn(move |req: Request<Bytes>| {
        let stream = stream.clone();
        async move { ipc_handler::handle_request(req, stream).await }
    });

    // Run a simple HTTP server over the TCP stream for IPC
    let _ = Server::builder()
        .http1_title_case(true)
        .serve_connection(stream)
        .await;
}

/// Parse and handle incoming IPC requests
async fn handle_request(req: Request<Bytes>, stream: TcpStream) -> Result<hyper::Response<hyper::Body>, hyper::Error> {
    use hyper::Method;

    let path = req.uri().path();
    let method = req.method();

    info!("IPC request: {} {}", method, path);

    match (method, path) {
        (&Method::GET, "/status") => {
            Ok(hyper::Response::new(
                hyper::Body::from(r#"{"status": "ok"}"#.into())
            ))
        }
        (&Method::POST, "/auth/challenge") => {
            ipc_handler::handle_auth_challenge(stream).await
        }
        _ => {
            Ok(hyper::Response::builder()
                .status(404)
                .body(hyper::Body::from("Not Found"))
                .unwrap())
        }
    }
}