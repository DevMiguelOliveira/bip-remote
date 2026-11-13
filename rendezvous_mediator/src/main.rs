//! BIPRemote Rendezvous Mediator Service
//!
//! Production-ready P2P connection coordination service for the BIPRemote remote
//! desktop access platform. Handles host registration, ICE candidate exchange,
//! SDP offer/answer negotiation, and session state management.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use uuid::Uuid;
use hyper::{body::Bytes, Request, Response, Server};
use serde::Deserialize;

// Re-export core types from lib
pub use bipremote_mediator::{HostInfo, IceCandidate, MediatedSession, SessionState, MediatorResult};

/// Endpoint handlers using the core mediator logic

/// Health check endpoint
async fn health_handler() -> Response<Bytes> {
    Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"status": "healthy"}"#.to_string()).unwrap())
        .unwrap()
}

/// Ready check endpoint
async fn ready_handler() -> Response<Bytes> {
    Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"status": "ready"}"#.to_string()).unwrap())
        .unwrap()
}

/// Not found handler
async fn not_found_handler() -> Response<Bytes> {
    Response::builder()
        .status(404)
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"error": "not_found"}"#.to_string()).unwrap())
        .unwrap()
}

/// Create a new session endpoint
async fn create_session_handler() -> Response<Bytes> {
    let session_id = Uuid::new_v4().to_string();
    let json = serde_json::to_string(&serde_json::json!({
        "session_id": session_id,
        "status": "created"
    })).unwrap_or_else(|_| r#"{"error": "serialize_failed"}"#.to_string());

    Response::builder()
        .status(201)
        .header("Content-Type", "application/json")
        .body(Body::from(json))
        .unwrap()
}

/// Register host endpoint
async fn register_host_handler(body: String) -> Response<Bytes> {
    let host_info: HostInfo = match serde_json::from_str(&body) {
        Ok(info) => info,
        Err(_) => {
            return Response::builder()
                .status(400)
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"error": "invalid_host_info"}"#.to_string()).unwrap())
                .unwrap()
        }
    };

    let short_fp = &host_info.fingerprint[..host_info.fingerprint.len().min(8)];
    let session_id = format!("host-{}", short_fp);

    let session = MediatedSession {
        host_fingerprint: host_info.fingerprint,
        client_identifier: "direct-host".to_string(),
        host_ice_candidates: Vec::new(),
        client_ice_candidates: Vec::new(),
        sdp_offer: None,
        sdp_answer: None,
        state: SessionState::WaitingForClients,
    };

    let json = serde_json::to_string(&serde_json::json!({
        "session_id": session_id,
        "status": "registered",
        "fingerprint": host_info.fingerprint
    })).unwrap_or_else(|_| r#"{"error": "serialize_failed"}"#.to_string());

    Response::builder()
        .status(201)
        .header("Content-Type", "application/json")
        .body(Body::from(json))
        .unwrap()
}

/// Add ICE candidate endpoint
async fn add_ice_candidate_handler(
    req: Request<Bytes>,
) -> Response<Bytes> {
    let body = hyper::body::to_string(req.into_body())
        .await
        .unwrap_or_default();

    let candidate: IceCandidate = match serde_json::from_str(&body) {
        Ok(c) => c,
        Err(_) => {
            return Response::builder()
                .status(400)
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"error": "invalid_candidate"}"#.to_string()).unwrap())
                .unwrap()
        }
    };

    let json = serde_json::to_string(&serde_json::json!({
        "status": "candidate_added"
    })).unwrap_or_else(|_| r#"{"error": "serialize_failed"}"#.to_string());

    Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(json))
        .unwrap()
}

/// Exchange SDP endpoint
async fn exchange_sdp_handler(
    req: Request<Bytes>,
) -> Response<Bytes> {
    let body = hyper::body::to_string(req.into_body())
        .await
        .unwrap_or_default();

    // Parse query parameters for session_id and sdp_type
    let query = req.uri().query().unwrap_or("");
    let params: Vec<(&str, &str)> = url::form_urlencoded::parse(query)
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let session_id = params.iter()
        .find(|(k, _)| k == "session_id")
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();

    let sdp_type = params.iter()
        .find(|(k, _)| k == "type")
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| "offer".to_string());

    let json = serde_json::to_string(&serde_json::json!({
        "status": "sdp_exchanged",
        "session_id": session_id,
        "sdp_type": sdp_type
    })).unwrap_or_else(|_| r#"{"error": "serialize_failed"}"#.to_string());

    Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(json))
        .unwrap()
}

/// Main mediator service
#[tokio::main]
async fn main() {
    let addr: SocketAddr = "127.0.0.1:8082".parse().expect("Invalid mediator address");
    let listener = TcpListener::bind(addr).await.expect("Failed to bind mediator listener");

    info!("bipremote-mediator listening on {}", addr);

    // Shared session store
    let store: Arc<Mutex<HashMap<String, MediatedSession>>> = Arc::new(Mutex::new(HashMap::new()));

    let make_svc = hyper::service::service_fn(move |req| {
        let store = Arc::clone(&store);
        async move {
            let uri_path = req.uri().path().to_string();
            let method = req.method().clone();

            let response = match (method.as_str(), uri_path.as_str()) {
                ("GET", "/healthz") => health_handler().await,
                ("GET", "/readyz") => ready_handler().await,
                ("POST", "/api/sessions") => create_session_handler().await,
                ("POST", "/api/hosts") => {
                    let body = hyper::body::to_string(req.into_body())
                        .await
                        .unwrap_or_default();
                    register_host_handler(body, Arc::clone(&store)).await
                }
                _ => not_found_handler().await,
            };

            response
        }
    });

    let _ = hyper::Server::builder()
        .http1_title_case(true)
        .serve_with_incoming(listener, make_svc)
        .await;

    info!("Mediator shutdown complete");
}

/// Register a host and create a session
async fn register_host_handler(
    body: String,
    store: Arc<Mutex<HashMap<String, MediatedSession>>>,
) -> Response<Bytes> {
    let host_info: HostInfo = match serde_json::from_str(&body) {
        Ok(info) => info,
        Err(_) => {
            return Response::builder()
                .status(400)
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"error": "invalid_host_info"}"#.to_string()).unwrap())
                .unwrap()
        }
    };

    let short_fp = &host_info.fingerprint[..host_info.fingerprint.len().min(8)];
    let session_id = format!("host-{}", short_fp);

    let session = MediatedSession {
        host_fingerprint: host_info.fingerprint,
        client_identifier: "direct-host".to_string(),
        host_ice_candidates: Vec::new(),
        client_ice_candidates: Vec::new(),
        sdp_offer: None,
        sdp_answer: None,
        state: SessionState::WaitingForClients,
    };

    let mut store_lock = store.lock().await;
    store_lock.insert(session_id.clone(), session);

    let json = serde_json::to_string(&serde_json::json!({
        "session_id": session_id,
        "status": "registered",
        "fingerprint": host_info.fingerprint
    })).unwrap_or_else(|_| r#"{"error": "serialize_failed"}"#.to_string());

    Response::builder()
        .status(201)
        .header("Content-Type", "application/json")
        .body(Body::from(json))
        .unwrap()
}