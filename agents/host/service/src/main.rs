//! BIPRemote Host Service - Privileged Service with Secure IPC
//!
//! This service provides privileged operations for the BIPRemote host,
//! including screen capture, input injection, and stream management.
//! All communication is secured via mTLS.

mod secure_ipc;
mod security;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{debug, error, info, warn};
use tracing_subscriber::FmtSubscriber;

use crate::secure_ipc::{SecureIpcConfig, SecureIpcServer};

#[tokio::main]
async fn main() {
    // Initialize tracing subscriber
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    info!("bipremote-host-service starting with secure IPC");

    // Create secure IPC server with default configuration
    let config = SecureIpcConfig::default();
    let server = SecureIpcServer::new(config)
        .await
        .expect("Failed to create secure IPC server");

    // Start the server (this runs forever)
    if let Err(e) = server.start().await {
        error!("Server error: {}", e);
        std::process::exit(1);
    }
}
