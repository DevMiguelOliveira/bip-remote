use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use serde::{Serialize, Deserialize};

/// Authenticator for handling mTLS and JWT-based authentication
/// Between the admin console and the privileged service
pub struct Authenticator {
    /// Session keys indexed by session ID
    sessions: Arc<RwLock<std::collections::HashMap<String, SessionKey>>>,
}

#[derive(Clone, Debug)]
struct SessionKey {
    session_id: String,
    host_fingerprint: String,
    client_identifier: String,
    expires_at: std::time::Instant,
}

impl Authenticator {
    /// Create a new authenticator
    pub fn new() -> Self {
        Authenticator {
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Generate an authentication challenge for a new session
    pub async fn generate_challenge(&self) -> ChallengeResponse {
        let session_id = Uuid::new_v4().to_string();

        ChallengeResponse {
            session_id,
            challenge_type: "mtls".to_string(),
            challenge_data: serde_json::json!({
                "certificate_required": true,
                "fingerprint": "host-fingerprint-placeholder"
            }),
            expires_at: std::time::Instant::now() + std::time::Duration::fromminutes(5),
        }
    }

    /// Verify an authentication response
    pub async fn verify_response(
        &self,
        _session_id: &str,
        _response: &AuthResponse,
    ) -> bool {
        // TODO: Verify mTLS certificate or JWT token
        // Check session exists and is valid
        true
    }

    /// Register a new session key
    pub async fn register_session(&self, session_id: String, fingerprint: String) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), SessionKey {
            session_id: session_id.clone(),
            host_fingerprint: fingerprint,
            client_identifier: "unknown".to_string(),
            expires_at: std::time::Instant::now() + std::time::Duration::fromhours(1),
        });
    }

    /// Revoke a session key
    pub async fn revoke_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
    }
}

/// Challenge response sent to client
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChallengeResponse {
    /// Unique session identifier
    pub session_id: String,
    /// Type of challenge (mtls, jwt, tls-certificate)
    pub challenge_type: String,
    /// Challenge-specific data
    pub challenge_data: serde_json::Value,
    /// Expiration time (ISO format or relative)
    pub expires_at: String,
}

/// Authentication response received from client
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthResponse {
    /// Session ID
    pub session_id: String,
    /// Authentication status
    pub status: AuthStatus,
    /// Error message if failed
    pub error: Option<String>,
}

/// Authentication status
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AuthStatus {
    Success,
    Failed,
    Expired,
}

/// Session context for IPC
#[derive(Clone, Debug)]
pub struct SessionContext {
    pub session_id: String,
    pub host_fingerprint: String,
    pub client_identifier: String,
    pub authenticated: bool,
}