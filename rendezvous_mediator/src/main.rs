use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{info, error, warn, debug};
use hyper::{Body, Request, Response, Server, Service, make_service};
use serde::{Serialize, Deserialize};

// Data structures for the mediator

/// Host registration information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostInfo {
    pub fingerprint: String,
    pub capabilities: Vec<String>,
    pub listen_port: u16,
    pub metadata: HashMap<String, String>,
}

/// ICE candidate for P2P connection
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IceCandidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_mline_index: Option<i32>,
    pub username_fragment: Option<String>,
    pub password: Option<String>,
}

/// Session information stored by the mediator
#[derive(Clone, Debug)]
pub struct MediatedSession {
    pub host_fingerprint: String,
    pub client_identifier: String,
    pub host_ice_candidates: Vec<IceCandidate>,
    pub client_ice_candidates: Vec<IceCandidate>,
    pub sdp_offer: Option<String>,
    pub sdp_answer: Option<String>,
    pub state: SessionState,
}

/// Session state machine
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SessionState {
    Initiated,
    WaitingForClients,
    ExchangeOffer,
    ExchangeAnswer,
    Connected,
    Failed,
    Relayed,
}

/// Error types for mediator operations
#[derive(Clone, Debug, thiserror::Error)]
pub enum MediatorError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Session already exists: {0}")]
    SessionExists(String),
    #[error("Invalid fingerprint")]
    InvalidFingerprint,
    #[error("Invalid SDP")]
    InvalidSDP,
    #[error("ICE candidate validation failed")]
    ICEValidationFailed,
    #[error("Mediator overloaded")]
    Overloaded,
}

/// Result type for mediator operations
pub type MediatorResult<T> = Result<T, MediatorError>;

/// In-memory session store (replace with Redis/PG in production)
type SessionStore = Arc<Mutex<HashMap<String, MediatedSession>>>;

/// Start the rendezvous mediator service
#[tokio::main]
async fn main() {
    // Initialize tracing
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    let addr: SocketAddr = "127.0.0.1:8082".parse().expect("Invalid mediator address");
    let listener = TcpListener::bind(addr).await
        .expect("Failed to bind mediator listener");

    info!("bipremote-mediator listening on {}", addr);
    info!("BIPRemote P2P connection coordination service");

    // Shared session store
    let store: SessionStore = Arc::new(Mutex::new(HashMap::new()));

    // Create service with shared state
    let make_service = make_service_fn(move |_| {
        let store = Arc::clone(&store);
        async move {
            Ok::<_, hyper::Error>(HyperMediatorService { store })
        }
    });

    // Run the HTTP server
    let _ = Server::builder()
        .http1_title_case(true)
        .serve_with_incoming(listener, make_service)
        .await;

    info!("Mediator shutdown complete");
}

/// Custom Hyper service for handling mediator HTTP requests
struct HyperMediatorService {
    store: SessionStore,
}

impl Service<Request<Body>> for HyperMediatorService {
    type Response = hyper::Response<Body>;
    type Error = hyper::Error;
    type Future = hyper::body::Full<Body>;

    fn call(&self, req: Request<Body>) -> Self::Future {
        let store = Arc::clone(&self.store);
        async move { handle_request(req, store).await }
    }
}

/// Handle incoming HTTP requests to the mediator
async fn handle_request(req: Request<Body>, store: SessionStore) -> hyper::Response<Body> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    info!("Mediator request: {} {}", method, path);

    // Route based on path and method
    let response = match (method.as_str(), path.as_str()) {
        // Session management endpoints
        ("POST", "/api/sessions") => handle_create_session(&req).await,
        ("GET", "/api/sessions/:id") => handle_get_session(&req).await,
        ("DELETE", "/api/sessions/:id") => handle_delete_session(&req).await,

        // Host registration
        ("POST", "/api/hosts") => register_host(&req).await,
        ("GET", "/api/hosts/:fingerprint") => get_host_info(&req).await,

        // ICE candidate exchange
        ("POST", "/api/candidates") => add_ice_candidate(&req).await,
        ("GET", "/api/candidates/:session_id") => get_ice_candidates(&req).await,

        // SDP exchange
        ("POST", "/api/sdp") => exchange_sdp(&req).await,
        ("GET", "/api/sdp/:session_id") => get_sdp(&req).await,

        // Health check
        ("GET", "/healthz") => health_response(),
        ("GET", "/readyz") => ready_response(),

        // Catch-all 404
        _ => not_found_response(),
    };

    // Add CORS headers for web clients
    response
        .map(|body| {
            hyper::Response::builder()
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS")
                .header("Access-Control-Allow-Headers", "Content-Type, Authorization")
                .body(body)
                .unwrap()
        })
        .unwrap_or_else(|e| {
            hyper::Response::builder()
                .status(500)
                .body(Body::from("Internal Error"))
                .unwrap()
        })
}

async fn handle_create_session(req: &Request<Body>) -> hyper::Response<Body> {
    let body = hyper::body::to_string(req.into_body())
        .await
        .unwrap_or_default();

    let session_id = uuid::Uuid::new_v4().to_string();

    let session = MediatedSession {
        host_fingerprint: "pending".to_string(),
        client_identifier: "unknown".to_string(),
        host_ice_candidates: Vec::new(),
        client_ice_candidates: Vec::new(),
        sdp_offer: None,
        sdp_answer: None,
        state: SessionState::Initiated,
    };

    let mut store = store.lock().await;
    store.insert(session_id.clone(), session);

    let json = serde_json::to_string(&serde_json::json!({
        "session_id": session_id,
        "status": "created"
    })).unwrap_or_else(|_| r#"{"error": "serialize_failed"}"#.to_string());

    hyper::Response::builder()
        .status(201)
        .header("Content-Type", "application/json")
        .body(Body::from(json))
        .unwrap()
}

async fn handle_get_session(req: &Request<Body>) -> hyper::Response<Body> {
    // Extract session ID from path
    let session_id = req.uri().path()
        .replace("/api/sessions/", "")
        .replace("/api/sessions", "");

    let store = store.lock().await;
    let session = match store.get(&session_id) {
        Some(s) => s.clone(),
        None => {
            return hyper::Response::builder()
                .status(404)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({"error": "session_not_found"})).unwrap_or_default()
                ))
                .unwrap()
        }
    };

    let json = serde_json::to_string(&serde_json::json!({
        "session_id": session_id,
        "host_fingerprint": session.host_fingerprint,
        "client_identifier": session.client_identifier,
        "state": format!("{:?}", session.state),
        "ice_candidates_host": session.host_ice_candidates.len(),
        "ice_candidates_client": session.client_ice_candidates.len(),
    })).unwrap_or_else(|_| r#"{"error": "serialize_failed"}"#.to_string());

    hyper::Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(json))
        .unwrap()
}

async fn handle_delete_session(req: &Request<Body>) -> hyper::Response<Body> {
    let session_id = req.uri().path()
        .replace("/api/sessions/", "")
        .replace("/api/sessions", "");

    let mut store = store.lock().await;
    let removed = store.remove(&session_id);

    let json = serde_json::to_string(&serde_json::json!({
        "session_id": session_id,
        "removed": removed.is_some()
    })).unwrap_or_else(|_| r#"{"error": "serialize_failed"}"#.to_string());

    hyper::Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(json))
        .unwrap()
}

async fn register_host(req: &Request<Body>) -> hyper::Response<Body> {
    let body = hyper::body::to_string(req.into_body())
        .await
        .unwrap_or_default();

    let host_info: HostInfo = match serde_json::from_str(&body) {
        Ok(info) => info,
        Err(_) => {
            return hyper::Response::builder()
                .status(400)
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"error": "invalid_host_info"}"#.to_string()))
                .unwrap()
        }
    };

    let session_id = format!("host-{}", host_info.fingerprint[:8]);

    let session = MediatedSession {
        host_fingerprint: host_info.fingerprint.clone(),
        client_identifier: "direct-host".to_string(),
        host_ice_candidates: Vec::new(),
        client_ice_candidates: Vec::new(),
        sdp_offer: None,
        sdp_answer: None,
        state: SessionState::WaitingForClients,
    };

    let mut store = store.lock().await;
    store.insert(session_id.clone(), session);

    let json = serde_json::to_string(&serde_json::json!({
        "session_id": session_id,
        "status": "registered",
        "fingerprint": host_info.fingerprint
    })).unwrap_or_else(|_| r#"{"error": "serialize_failed"}"#.to_string());

    hyper::Response::builder()
        .status(201)
        .header("Content-Type", "application/json")
        .body(Body::from(json))
        .unwrap()
}

async fn get_host_info(_req: &Request<Body>) -> hyper::Response<Body> {
    // In a full implementation, would look up host by fingerprint
    hyper::Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"status": "ok", "message": "host_info_endpoint"}"#.to_string()))
        .unwrap()
}

async fn add_ice_candidate(req: &Request<Body>) -> hyper::Response<Body> {
    let body = hyper::body::to_string(req.into_body())
        .await
        .unwrap_or_default();

    let candidate: IceCandidate = match serde_json::from_str(&body) {
        Ok(c) => c,
        Err(_) => {
            return hyper::Response::builder()
                .status(400)
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"error": "invalid_candidate"}"#.to_string()))
                .unwrap()
        }
    };

    // Extract session ID from path: /api/candidates/:session_id
    let path = req.uri().path();
    let session_id = path.splitn(2, "/api/candidates/")
        .nth(1)
        .unwrap_or("");

    let mut store = store.lock().await;
    let session = match store.get_mut(session_id) {
        Some(s) => s,
        None => {
            return hyper::Response::builder()
                .status(404)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({"error": "session_not_found"})).unwrap_or_default()
                ))
                .unwrap()
        }
    };

    // Determine if this is host or client candidate based on session state
    // For now, add to host candidates if session was created by host registration
    if session.host_fingerprint != "pending" {
        session.host_ice_candidates.push(candidate);
    } else {
        session.client_ice_candidates.push(candidate);
    }

    let json = serde_json::to_string(&serde_json::json!({
        "status": "candidate_added",
        "session_id": session_id,
        "candidates_count": session.host_ice_candidates.len() + session.client_ice_candidates.len()
    })).unwrap_or_else(|_| r#"{"error": "serialize_failed"}"#.to_string());

    hyper::Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(json))
        .unwrap()
}

async fn get_ice_candidates(req: &Request<Body>) -> hyper::Response<Body> {
    // Extract session ID from path
    let path = req.uri().path();
    let session_id = path.splitn(2, "/api/candidates/")
        .nth(1)
        .unwrap_or("");

    let store = store.lock().await;
    let session = match store.get(session_id) {
        Some(s) => s.clone(),
        None => {
            return hyper::Response::builder()
                .status(404)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({"error": "session_not_found"})).unwrap_or_default()
                ))
                .unwrap()
        }
    };

    let json = serde_json::to_string(&serde_json::json!({
        "session_id": session_id,
        "host_candidates": session.host_ice_candidates.len(),
        "client_candidates": session.client_ice_candidates.len(),
    })).unwrap_or_else(|_| r#"{"error": "serialize_failed"}"#.to_string());

    hyper::Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(json))
        .unwrap()
}

async fn exchange_sdp(req: &Request<body::Bytes>) -> hyper::Response<Body> {
    let body = hyper::body::to_string(req.into_body())
        .await
        .unwrap_or_default();

    // Extract session ID from path: /api/sdp/:session_id
    let path = req.uri().path();
    let session_id = path.splitn(2, "/api/sdp/")
        .nth(1)
        .unwrap_or("");

    let sdp_type = req
        .uri()
        .query()
        .map(|q| {
            url::form_urlencoded::parse(q)
                .find(|(k, _)| k == "type")
                .map(|(_, v)| v.to_string())
                .unwrap_or_else(|| "offer".to_string())
        })
        .unwrap_or_else(|| "offer".to_string());

    let mut store = store.lock().await;
    let session = match store.get_mut(session_id) {
        Some(s) => s,
        None => {
            return hyper::Response::builder()
                .status(404)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({"error": "session_not_found"})).unwrap_or_default()
                ))
                .unwrap()
        }
    };

    match session_id.parse::<uuid::Uuid>() {
        Ok(_) => {}
        Err(_) => {
            return hyper::Response::builder()
                .status(400)
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"error": "invalid_session_id"}"#.to_string()))
                .unwrap()
        }
    }

    match session_type_to_enum(&session.state) {
        SessionState::Initiated => {
            if sdp_type == "offer" {
                session.sdp_offer = Some(body.clone());
                session.state = SessionState::ExchangeOffer;
            }
        }
        SessionState::ExchangeOffer => {
            if sdp_type == "answer" {
                session.sdp_answer = Some(body.clone());
                session.state = SessionState::ExchangeAnswer;
            }
        }
        _ => {}
    }

    let json = serde_json::to_string(&serde_json::json!({
        "status": "sdp_exchanged",
        "session_id": session_id,
        "state": format!("{:?}", session.state)
    })).unwrap_or_else(|_| r#"{"error": "serialize_failed"}"#.to_string());

    hyper::Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(json))
        .unwrap()
}

async fn get_sdp(req: &Request<Body>) -> hyper::Response<Body> {
    // Extract session ID from path
    let path = req.uri().path();
    let session_id = path.splitn(2, "/api/sdp/")
        .nth(1)
        .unwrap_or("");

    let store = store.lock().await;
    let session = match store.get(session_id) {
        Some(s) => s.clone(),
        None => {
            return hyper::Response::builder()
                .status(404)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({"error": "session_not_found"})).unwrap_or_default()
                ))
                .unwrap()
        }
    };

    let sdp_type = req
        .uri()
        .query()
        .map(|q| {
            url::form_urlencoded::parse(q)
                .find(|(k, _)| k == "type")
                .map(|(_, v)| v.to_string())
                .unwrap_or_else(|| "offer".to_string())
        })
        .unwrap_or_else(|| "offer".to_string());

    let sdp_value = match sdp_type.as_str() {
        "offer" => session.sdp_offer.clone().unwrap_or_default(),
        "answer" => session.sdp_answer.clone().unwrap_or_default(),
        _ => session.sdp_offer.clone().unwrap_or_default(),
    };

    let json = serde_json::to_string(&serde_json::json!({
        "session_id": session_id,
        "sdp_type": sdp_type,
        "sdp": sdp_value,
        "has_sdp": !sdp_value.is_empty()
    })).unwrap_or_else(|_| r#"{"error": "serialize_failed"}"#.to_string());

    hyper::Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(json))
        .unwrap()
}

async fn handle_unknown(_: &Request<Body>) -> hyper::Response<Body> {
    not_found_response()
}

fn session_type_to_enum(state: &SessionState) -> SessionState {
    // Helper to convert - in real impl would just use the state directly
    state.clone()
}

fn health_response() -> hyper::Response<Body> {
    hyper::Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"status": "healthy"}"#.to_string()))
        .unwrap()
}

fn ready_response() -> hyper::Response<Body> {
    hyper::Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"status": "ready"}"#.to_string()))
        .unwrap()
}

fn not_found_response() -> hyper::Response<Body> {
    hyper::Response::builder()
        .status(404)
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"error": "not_found"}"#.to_string())
        .unwrap())
}