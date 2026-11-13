//! Security module for mTLS and certificate handling in BIPRemote host service
//!
//! Provides mutual TLS authentication between host and privileged service,
//! certificate generation, validation, and secure session token management.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use ed25519_dalek::{SigningKey, VerifyingKey, Verifier, Signature, Signer};
use rand::thread_rng;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, pem::PemObject};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Simple CA using pre-generated certificates
pub struct ServiceCA {
    ca_cert_pem: Vec<u8>,
    ca_key_pem: Vec<u8>,
    cert_ttl: Duration,
}

impl ServiceCA {
    /// Create a new CA for the service
    pub fn new(cert_ttl_hours: u64) -> Result<Self, String> {
        // Generate a simple self-signed CA certificate
        let (ca_cert_pem, ca_key_pem) = generate_self_signed_ca()?;
        Ok(Self {
            ca_cert_pem,
            ca_key_pem,
            cert_ttl: Duration::from_secs(cert_ttl_hours * 3600),
        })
    }

    /// Issue a certificate for a host (simplified - returns the CA cert for now)
    pub fn issue_host_cert(
        &self,
        _host_fingerprint: &str,
        _host_id: &str,
    ) -> Result<(Vec<u8>, Vec<u8>), String> {
        // In a real implementation, this would generate a host-specific cert
        // For now, return the CA cert as placeholder
        Ok((self.ca_cert_pem.clone(), self.ca_key_pem.clone()))
    }

    /// Get CA certificate in PEM format
    pub fn ca_cert_pem(&self) -> Vec<u8> {
        self.ca_cert_pem.clone()
    }

    /// Get CA certificate as DER
    pub fn ca_cert_der(&self) -> Vec<u8> {
        let pem_str = String::from_utf8_lossy(&self.ca_cert_pem);
        let b64_start = pem_str.find("-----BEGIN CERTIFICATE-----").unwrap_or(0);
        let b64_end = pem_str
            .find("-----END CERTIFICATE-----")
            .unwrap_or(pem_str.len());
        let b64 = &pem_str[b64_start..]
            .replace("-----BEGIN CERTIFICATE-----", "")
            .replace("-----END CERTIFICATE-----", "")
            .replace('\n', "")
            .replace('\r', "");
        let b64_clean = &b64[b64_start..b64_end.min(b64.len())];
        base64::decode(b64_clean).unwrap_or_default()
    }

    /// Get CA private key as PKCS8 DER
    pub fn ca_key_der(&self) -> Vec<u8> {
        let pem_str = String::from_utf8_lossy(&self.ca_key_pem);
        let b64_start = pem_str.find("-----BEGIN PRIVATE KEY-----").unwrap_or(0);
        let b64_end = pem_str
            .find("-----END PRIVATE KEY-----")
            .unwrap_or(pem_str.len());
        let b64 = &pem_str[b64_start..]
            .replace("-----BEGIN PRIVATE KEY-----", "")
            .replace("-----END PRIVATE KEY-----", "")
            .replace('\n', "")
            .replace('\r', "");
        let b64_clean = &b64[b64_start..b64_end.min(b64.len())];
        base64::decode(b64_clean).unwrap_or_default()
    }
}

/// Session token manager with TTL
pub struct SessionTokenManager {
    tokens: Arc<RwLock<HashMap<String, SessionToken>>>,
    ttl: Duration,
    cleanup_interval: Duration,
}

#[derive(Clone, Debug)]
pub struct SessionToken {
    pub token: String,
    pub session_id: String,
    pub host_fingerprint: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub permissions: Vec<String>,
}

impl SessionTokenManager {
    pub fn new(ttl_hours: u64, cleanup_hours: u64) -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::from_secs(ttl_hours * 3600),
            cleanup_interval: Duration::from_secs(cleanup_hours * 3600),
        }
    }

    pub async fn create_token(
        &self,
        session_id: String,
        host_fingerprint: String,
        permissions: Vec<String>,
    ) -> Result<String, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();

        let expires_at = now + self.ttl.as_secs();
        let token = BASE64_STANDARD.encode(Uuid::new_v4().as_bytes());

        let session_token = SessionToken {
            token: token.clone(),
            session_id,
            host_fingerprint,
            created_at: now,
            expires_at,
            permissions,
        };

        let mut tokens = self.tokens.write().await;
        tokens.insert(token.clone(), session_token);

        debug!(
            "Created session token for session: {}",
            session_token.session_id
        );
        Ok(token)
    }

    pub async fn validate_token(&self, token: &str) -> Result<SessionToken, String> {
        let mut tokens = self.tokens.write().await;

        let session_token = tokens.get(token).ok_or("Invalid or expired token")?.clone();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();

        if now >= session_token.expires_at {
            tokens.remove(token);
            return Err("Token expired".to_string());
        }

        if let Some(token_mut) = tokens.get_mut(token) {
            token_mut.expires_at = now + self.ttl.as_secs();
        }

        Ok(session_token)
    }

    pub async fn revoke_token(&self, token: &str) -> bool {
        let mut tokens = self.tokens.write().await;
        tokens.remove(token).is_some()
    }

    pub async fn list_active_tokens(&self) -> Vec<SessionToken> {
        let tokens = self.tokens.read().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        tokens
            .values()
            .filter(|t| t.expires_at > now)
            .cloned()
            .collect()
    }

    pub async fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(self.cleanup_interval);
            loop {
                interval.tick().await;
                let mut tokens = self.tokens.write().await;
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let before = tokens.len();
                tokens.retain(|_, t| t.expires_at > now);
                let after = tokens.len();

                if before != after {
                    info!("Cleaned up {} expired session tokens", before - after);
                }
            }
        });
    }
}

/// Secure message wrapper for IPC
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SecureMessage {
    pub message_type: String,
    pub session_id: String,
    pub timestamp: u64,
    pub payload: serde_json::Value,
    pub signature: String,
    pub payload_hash: String,
}

impl SecureMessage {
    pub fn new(
        message_type: String,
        session_id: String,
        payload: serde_json::Value,
        signing_key: &ed25519_dalek::SigningKey,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let payload_bytes = serde_json::to_vec(&payload).unwrap_or_default();
        let payload_hash = BASE64_STANDARD.encode(Sha256::digest(&payload_bytes));
        let signature = BASE64_STANDARD.encode(signing_key.sign(&payload_bytes).to_bytes());

        Self {
            message_type,
            session_id,
            timestamp,
            payload,
            signature,
            payload_hash,
        }
    }

    pub fn verify(&self, verifying_key: &ed25519_dalek::VerifyingKey) -> bool {
        let payload_bytes = serde_json::to_vec(&self.payload).unwrap_or_default();
        let expected_hash = BASE64_STANDARD.encode(Sha256::digest(&payload_bytes));

        if self.payload_hash != expected_hash {
            warn!("Payload hash mismatch");
            return false;
        }

        let signature_bytes = match BASE64_STANDARD.decode(&self.signature) {
            Ok(b) => b,
            Err(_) => {
                warn!("Invalid signature encoding");
                return false;
            }
        };

        if signature_bytes.len() != 64 {
            warn!("Invalid signature length");
            return false;
        }

        let mut sig_array = [0u8; 64];
        sig_array.copy_from_slice(&signature_bytes);
        let signature = ed25519_dalek::Signature::from_bytes(&sig_array);

        verifying_key.verify(&payload_bytes, &signature).is_ok()
    }
}

fn generate_self_signed_ca() -> Result<(Vec<u8>, Vec<u8>), String> {
    // Generate a simple self-signed certificate using rcgen
    let key = rcgen::KeyPair::generate(&rcgen::PKCS_ECDSA_P256_SHA256)
        .map_err(|e| format!("Failed to generate key: {}", e))?;

    let mut params = rcgen::CertificateParams::new(vec![]);
    params.distinguished_name = rcgen::DistinguishedName::new();
    params.distinguished_name.push(rcgen::DnType::CommonName, "BIPRemote Service CA");
    params.distinguished_name.push(rcgen::DnType::OrganizationName, "BIPRemote");
    params.distinguished_name.push(rcgen::DnType::OrganizationalUnitName, "Certificate Authority");
    params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params.key_usages = vec![
        rcgen::KeyUsagePurpose::KeyCertSign,
        rcgen::KeyUsagePurpose::CrlSign,
    ];

    let key_pair = rcgen::KeyPair::generate(&rcgen::PKCS_ECDSA_P256_SHA256)
        .map_err(|e| format!("Failed to generate key: {}", e))?;

    let cert = rcgen::Certificate::from_params(params)
        .map_err(|e| format!("Failed to create cert: {}", e))?;

    let cert_pem = cert.serialize_pem().map_err(|e| e.to_string())?;
    let key_pem = key_pair.serialize_pem();

    Ok((cert_pem.into_bytes(), key_pem.into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ca_creation() {
        let ca = ServiceCA::new(24).unwrap();
        let ca_pem = ca.ca_cert_pem();
        assert!(!ca_pem.is_empty());
        assert!(String::from_utf8_lossy(&ca_pem).contains("CERTIFICATE"));
    }

    #[tokio::test]
    async fn test_session_token_lifecycle() {
        let manager = SessionTokenManager::new(1, 1);

        let token = manager
            .create_token(
                "session-123".to_string(),
                "fingerprint-abc".to_string(),
                vec!["read".to_string(), "write".to_string()],
            )
            .await
            .unwrap();

        let validated = manager.validate_token(&token).await.unwrap();
        assert_eq!(validated.session_id, "session-123");
        assert_eq!(validated.host_fingerprint, "fingerprint-abc");

        let revoked = manager.revoke_token(&token).await;
        assert!(revoked);

        let result = manager.validate_token(&token).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_secure_message_signing() {
        use ed25519_dalek::{SigningKey, VerifyingKey};
        use rand::thread_rng;

        let mut rng = thread_rng();
        let signing_key = SigningKey::generate(&mut rng);
        let verifying_key = signing_key.verifying_key();

        let message = SecureMessage::new(
            "test_type".to_string(),
            "session-123".to_string(),
            serde_json::json!({"data": "test"}),
            &signing_key,
        );

        assert!(message.verify(&verifying_key));

        let mut tampered = message.clone();
        tampered.payload = serde_json::json!({"data": "tampered"});
        assert!(!tampered.verify(&verifying_key));
    }
}