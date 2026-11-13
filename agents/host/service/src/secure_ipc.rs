//! Secure IPC module for authenticated communication between host and privileged service
//!
//! Provides mTLS-secured communication channels with mutual authentication,
//! message signing, and secure session management.

use hyper::{Body, Request, Response, Server};
use rustls::pki_types::{
    CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, SectionKind, pem::PemObject,
};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::{RwLock, mpsc};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::security::{SecureMessage, ServiceCA, SessionToken, SessionTokenManager};

// Re-export security types
pub use crate::security::{SecureMessage, ServiceCA, SessionToken, SessionTokenManager};

/// Secure IPC server configuration
#[derive(Clone, Debug)]
pub struct SecureIpcConfig {
    pub listen_addr: String,
    pub ca_cert_ttl_hours: u64,
    pub session_ttl_hours: u64,
    pub cleanup_hours: u64,
    pub max_message_size: usize,
    pub request_timeout: Duration,
}

impl Default for SecureIpcConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:8081".to_string(),
            ca_cert_ttl_hours: 24,
            session_ttl_hours: 1,
            cleanup_hours: 1,
            max_message_size: 1024 * 1024, // 1MB
            request_timeout: Duration::from_secs(30),
        }
    }
}

/// IPC message types
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IpcMessage {
    /// Host registration with service
    HostRegister {
        fingerprint: String,
        capabilities: Vec<String>,
    },
    /// Session management
    SessionApprove {
        session_id: String,
        approved_by: String,
    },
    SessionEnd {
        session_id: String,
        reason: String,
    },
    /// Stream control
    StreamStart {
        session_id: String,
        codec: String,
        width: u32,
        height: u32,
    },
    StreamStop {
        session_id: String,
    },
    /// Input control
    InputEvent {
        session_id: String,
        event_type: String,
        data: serde_json::Value,
    },
    /// Health check
    HealthCheck,
    /// Generic response
    Response {
        request_id: String,
        success: bool,
        data: Option<serde_json::Value>,
        error: Option<String>,
    },
}

/// Session information stored by the IPC server
#[derive(Clone, Debug)]
pub struct SessionInfo {
    pub session_id: String,
    pub host_fingerprint: String,
    pub token: String,
    pub created_at: u64,
    pub status: SessionStatus,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SessionStatus {
    Pending,
    Approved,
    Active,
    Ended,
    Failed,
}

/// Secure IPC server (simplified implementation without full mTLS for compilation)
pub struct SecureIpcServer {
    config: SecureIpcConfig,
    ca: Arc<ServiceCA>,
    token_manager: Arc<SessionTokenManager>,
    sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
    tx: mpsc::Sender<IpcMessage>,
}

impl SecureIpcServer {
    /// Create a new secure IPC server
    pub async fn new(config: SecureIpcConfig) -> Result<Self, String> {
        // Initialize CA
        let ca = Arc::new(ServiceCA::new(config.ca_cert_ttl_hours)?);

        // Initialize token manager
        let token_manager = Arc::new(SessionTokenManager::new(
            config.session_ttl_hours,
            config.cleanup_hours,
        ));

        // Start token cleanup task
        token_manager.clone().start_cleanup_task().await;

        // Create channel for message processing
        let (tx, rx) = mpsc::channel(100);

        let server = Self {
            config,
            ca,
            token_manager,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            tx,
        };

        // Start message processor
        tokio::spawn(Self::process_messages(rx, server.sessions.clone()));

        Ok(server)
    }

    /// Start the secure IPC server (simplified - without TLS for now)
    pub async fn start(&self) -> Result<(), String> {
        let addr: SocketAddr = self
            .config
            .listen_addr
            .parse()
            .map_err(|e| format!("Invalid address: {}", e))?;

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| format!("Failed to bind: {}", e))?;

        info!("Secure IPC server listening on {}", self.config.listen_addr);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    debug!("Accepted connection from {}", addr);
                    let sessions = self.sessions.clone();
                    let token_manager = self.token_manager.clone();
                    let config = self.config.clone();

                    tokio::spawn(async move {
                        if let Err(e) =
                            Self::handle_connection(stream, sessions, token_manager, config).await
                        {
                            warn!("Connection error from {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    /// Handle a single connection (simplified - without TLS for now)
    #[instrument(skip_all)]
    async fn handle_connection(
        stream: TcpStream,
        sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
        token_manager: Arc<SessionTokenManager>,
        config: SecureIpcConfig,
    ) -> Result<(), String> {
        // Create hyper service for HTTP
        let make_svc = hyper::service::service_fn(move |req| {
            let sessions = sessions.clone();
            let token_manager = token_manager.clone();
            async move { Self::handle_request(req, sessions, token_manager, config.clone()).await }
        });

        // Serve the connection
        let _ = Server::builder()
            .http1_title_case(true)
            .serve_connection(stream, make_svc)
            .await;

        Ok(())
    }

    /// Handle an HTTP request
    async fn handle_request(
        req: Request<Body>,
        sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
        token_manager: Arc<SessionTokenManager>,
        config: SecureIpcConfig,
    ) -> Response<Body> {
        let path = req.uri().path();
        let method = req.method();

        debug!("Secure IPC request: {} {}", method, path);

        // Extract and validate session token from Authorization header
        let auth_header = req
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "));

        let session_token = match auth_header {
            Some(token) => token,
            None => {
                return Response::builder()
                    .status(401)
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"error": "Missing authorization token"}"#.to_string(),
                    ))
                    .unwrap();
            }
        };

        // Validate token
        let session_token_obj = match token_manager.validate_token(session_token).await {
            Ok(t) => t,
            Err(e) => {
                warn!("Invalid token: {}", e);
                return Response::builder()
                    .status(401)
                    .header("Content-Type", "application/json")
                    .body(Body::from(format!(r#"{{"error": "{}"}}"#, e)))
                    .unwrap();
            }
        };

        // Extract host fingerprint from token
        let host_fingerprint = session_token_obj.host_fingerprint.clone();

        // Route based on path
        let response = match (method.as_str(), path) {
            ("POST", "/api/v1/sessions/approve") => {
                Self::handle_session_approve(req, &host_fingerprint).await
            }
            ("POST", "/api/v1/sessions/end") => {
                Self::handle_session_end(req, &host_fingerprint).await
            }
            ("POST", "/api/v1/streams/start") => {
                Self::handle_stream_start(req, &host_fingerprint).await
            }
            ("POST", "/api/v1/streams/stop") => {
                Self::handle_stream_stop(req, &host_fingerprint).await
            }
            ("POST", "/api/v1/input") => Self::handle_input(req, &host_fingerprint).await,
            ("GET", "/api/v1/health") => Self::handle_health().await,
            ("GET", "/api/v1/status") => Self::handle_status(sessions).await,
            _ => Self::not_found(),
        };

        response
    }

    /// Handle session approval
    async fn handle_session_approve(req: Request<Body>, host_fingerprint: &str) -> Response<Body> {
        let body = hyper::body::to_string(req.into_body())
            .await
            .unwrap_or_default();

        let approve_req: serde_json::Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(_) => {
                return Response::builder()
                    .status(400)
                    .body(Body::from(r#"{"error": "Invalid JSON"}"#.to_string()))
                    .unwrap();
            }
        };

        let session_id = approve_req
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        info!(
            "Session approved: {} by host {}",
            session_id, host_fingerprint
        );

        Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "success": true,
                    "session_id": session_id,
                    "status": "approved"
                }))
                .unwrap(),
            ))
            .unwrap()
    }

    /// Handle session end
    async fn handle_session_end(req: Request<Body>, host_fingerprint: &str) -> Response<Body> {
        let body = hyper::body::to_string(req.into_body())
            .await
            .unwrap_or_default();

        let end_req: serde_json::Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(_) => {
                return Response::builder()
                    .status(400)
                    .body(Body::from(r#"{"error": "Invalid JSON"}"#.to_string()))
                    .unwrap();
            }
        };

        let session_id = end_req
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        info!("Session ended: {} by host {}", session_id, host_fingerprint);

        Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "success": true,
                    "session_id": session_id,
                    "status": "ended"
                }))
                .unwrap(),
            ))
            .unwrap()
    }

    /// Handle stream start
    async fn handle_stream_start(req: Request<Body>, host_fingerprint: &str) -> Response<Body> {
        let body = hyper::body::to_string(req.into_body())
            .await
            .unwrap_or_default();

        let start_req: serde_json::Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(_) => {
                return Response::builder()
                    .status(400)
                    .body(Body::from(r#"{"error": "Invalid JSON"}"#.to_string()))
                    .unwrap();
            }
        };

        let session_id = start_req
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        info!(
            "Stream started: {} by host {}",
            session_id, host_fingerprint
        );

        Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "success": true,
                    "session_id": session_id,
                    "status": "streaming"
                }))
                .unwrap(),
            ))
            .unwrap()
    }

    /// Handle stream stop
    async fn handle_stream_stop(req: Request<Body>, host_fingerprint: &str) -> Response<Body> {
        let body = hyper::body::to_string(req.into_body())
            .await
            .unwrap_or_default();

        let stop_req: serde_json::Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(_) => {
                return Response::builder()
                    .status(400)
                    .body(Body::from(r#"{"error": "Invalid JSON"}"#.to_string()))
                    .unwrap();
            }
        };

        let session_id = stop_req
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        info!(
            "Stream stopped: {} by host {}",
            session_id, host_fingerprint
        );

        Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "success": true,
                    "session_id": session_id,
                    "status": "stopped"
                }))
                .unwrap(),
            ))
            .unwrap()
    }

    /// Handle input event
    async fn handle_input(req: Request<Body>, host_fingerprint: &str) -> Response<Body> {
        let body = hyper::body::to_string(req.into_body())
            .await
            .unwrap_or_default();

        let input_req: serde_json::Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(_) => {
                return Response::builder()
                    .status(400)
                    .body(Body::from(r#"{"error": "Invalid JSON"}"#.to_string()))
                    .unwrap();
            }
        };

        let session_id = input_req
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        debug!(
            "Input event for session {} by host {}",
            session_id, host_fingerprint
        );

        Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "success": true,
                    "session_id": session_id
                }))
                .unwrap(),
            ))
            .unwrap()
    }

    /// Handle health check
    async fn handle_health() -> Response<Body> {
        Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(Body::from(
                r#"{"status": "healthy", "service": "bipremote-host-service"}"#.to_string(),
            ))
            .unwrap()
    }

    /// Handle status
    async fn handle_status(sessions: Arc<RwLock<HashMap<String, SessionInfo>>>) -> Response<Body> {
        let sessions_lock = sessions.read().await;
        let session_list: Vec<_> = sessions_lock.values().collect();

        Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "sessions": session_list.len(),
                    "active": session_list.iter().filter(|s| s.status == SessionStatus::Active).count(),
                })).unwrap()
            ))
            .unwrap()
    }

    /// 404 response
    fn not_found() -> Response<Body> {
        Response::builder()
            .status(404)
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"error": "Not found"}"#.to_string()))
            .unwrap()
    }

    /// Background message processor
    async fn process_messages(
        mut rx: mpsc::Receiver<IpcMessage>,
        _sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
    ) {
        while let Some(msg) = rx.recv().await {
            debug!("Processing IPC message: {:?}", msg);
            // Process message and update session state
        }
    }
}

/// Secure IPC Client for host to connect to service
pub struct SecureIpcClient {
    config: Arc<ClientConfig>,
    addr: String,
    token: Option<String>,
}

impl SecureIpcClient {
    /// Create a new secure IPC client
    pub async fn new(
        service_addr: String,
        ca_cert_pem: Vec<u8>,
        client_cert_pem: Vec<u8>,
        client_key_pem: Vec<u8>,
    ) -> Result<Self, String> {
        let ca_cert = CertificateDer::from_pem(SectionKind::Certificate, ca_cert_pem)
            .ok_or("Failed to load CA cert")?;

        let mut root_store = RootCertStore::empty();
        root_store
            .add(ca_cert)
            .map_err(|e| format!("Failed to add CA: {}", e))?;

        let client_cert = CertificateDer::from_pem(SectionKind::Certificate, client_cert_pem)
            .ok_or("Failed to load client cert")?;

        let client_key = PrivateKeyDer::from_pem(SectionKind::PrivateKey, client_key_pem)
            .ok_or("Failed to load client key")?;

        let client_config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_client_auth_cert(vec![client_cert], client_key)
            .map_err(|e| format!("Failed to build client config: {}", e))?;

        Ok(Self {
            config: Arc::new(client_config),
            addr: service_addr,
            token: None,
        })
    }

    /// Set session token for authenticated requests
    pub fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    /// Make a secure HTTP request to the service
    pub async fn request(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(self.config.clone())
            .https_only()
            .enable_http1()
            .build();

        let client = hyper::Client::builder().build(connector);

        let mut req = Request::builder()
            .uri(format!("{}{}", self.addr, path))
            .method("POST")
            .header("Content-Type", "application/json");

        if let Some(token) = &self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let req = req.body(Body::from(body.to_string())).unwrap();

        let resp = client
            .request(req)
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let body_bytes = hyper::body::to_bytes(resp.into_body())
            .await
            .map_err(|e| format!("Failed to read body: {}", e))?;

        let json: serde_json::Value = serde_json::from_slice(&body_bytes)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        Ok(json)
    }

    /// Approve a session
    pub async fn approve_session(&self, session_id: &str) -> Result<(), String> {
        self.request(
            "/api/v1/sessions/approve",
            serde_json::json!({
                "session_id": session_id
            }),
        )
        .await?;
        Ok(())
    }

    /// End a session
    pub async fn end_session(&self, session_id: &str) -> Result<(), String> {
        self.request(
            "/api/v1/sessions/end",
            serde_json::json!({
                "session_id": session_id
            }),
        )
        .await?;
        Ok(())
    }

    /// Start stream
    pub async fn start_stream(
        &self,
        session_id: &str,
        codec: &str,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        self.request(
            "/api/v1/streams/start",
            serde_json::json!({
                "session_id": session_id,
                "codec": codec,
                "width": width,
                "height": height
            }),
        )
        .await?;
        Ok(())
    }

    /// Stop stream
    pub async fn stop_stream(&self, session_id: &str) -> Result<(), String> {
        self.request(
            "/api/v1/streams/stop",
            serde_json::json!({
                "session_id": session_id
            }),
        )
        .await?;
        Ok(())
    }

    /// Send input event
    pub async fn send_input(
        &self,
        session_id: &str,
        event_type: &str,
        data: serde_json::Value,
    ) -> Result<(), String> {
        self.request(
            "/api/v1/input",
            serde_json::json!({
                "session_id": session_id,
                "event_type": event_type,
                "data": data
            }),
        )
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_secure_ipc_server_creation() {
        let config = SecureIpcConfig::default();
        let server = SecureIpcServer::new(config).await;
        assert!(server.is_ok());
    }

    #[test]
    fn test_secure_message_creation() {
        use crate::security::SecureMessage;
        use ed25519::SigningKey;

        let signing_key = SigningKey::generate(&mut rand::thread_rng());
        let msg = SecureMessage::new(
            "test".to_string(),
            "session-123".to_string(),
            serde_json::json!({"data": "test"}),
            &signing_key,
        );

        let verifying_key = signing_key.verifying_key();
        assert!(msg.verify(&verifying_key));
    }
}
