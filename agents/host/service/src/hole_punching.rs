use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};
use tracing::{info, error, warn, debug};
use serde::{Serialize, Deserialize};
use rand::Rng;
use reqwest::Client;
use url::Url;

// Hole punching state types
#[derive(Clone, Debug)]
pub struct HolePunchingState {
    /// Our fingerprint/identity
    pub host_fingerprint: String,
    /// Our listening port for P2P
    pub local_port: u16,
    /// Remote client fingerprint
    pub client_fingerprint: String,
    /// Remote client port they're listening on
    pub client_port: u16,
    /// ICE/TLS candidates exchanged
    pub candidates: Vec<Candidate>,
    /// Connection state
    pub state: HolePunchingStateEnum,
    /// Relay URL as fallback
    pub relay_url: Option<String>,
}

/// Candidate type for ICE
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub component: u16,
    pub foundation: String,
    pub priority: u32,
    pub ip: String,
    pub protocol: String,
    pub port: u16,
    /// Type: "host", "srflx", "relay", "prflx"
    pub candidate_type: String,
    /// Network type: "udp", "tcp"
    tcp_type: Option<String>,
}

/// Hole punching state enum
#[derive(Clone, Debug, PartialEq)]
pub enum HolePunchingStateEnum {
    Initiated,
    Waiting,
    Checking,
    Connected,
    Failed,
    Relayed,
}

/// Session setup result
#[derive(Clone, Debug, Serialize)]
pub struct HolePunchResult {
    pub success: bool,
    pub session_id: String,
    pub transport: String, // "p2p" or "relayed"
    pub relay_url: Option<String>,
    pub duration_ms: u64,
}

impl HolePunchingState {
    pub fn new(host_fingerprint: String, local_port: u16) -> Self {
        HolePunchingState {
            host_fingerprint,
            local_port,
            client_fingerprint: String::new(),
            client_port: 0,
            candidates: Vec::new(),
            state: HolePunchingStateEnum::Initiated,
            relay_url: None,
        }
    }

    /// Start the hole punching protocol
    pub async fn start(
        &mut self,
        mediator_url: &str,
        client_id: &str,
    ) -> Result<HolePunchResult, String> {
        info!("Starting TCP hole punching protocol");
        info!("Host fingerprint: {}", self.host_fingerprint);
        info!("Mediator: {}", mediator_url);

        // Phase 1: Contact mediator
        self.contact_mediator(mediator_url, client_id).await?;

        // Phase 2: Check for candidates and attempt P2P
        self.attempt_p2p().await?;

        // Phase 3: If P2P fails, set up relay
        if self.state != HolePunchingStateEnum::Connected {
            self.setup_relay().await?;
        }

        Ok(HolePunchResult {
            success: self.state == HolePunchingStateEnum::Connected,
            session_id: format!("session-{}", rand::thread_rng().gen::<u64>()),
            transport: self.state.transport_type(),
            relay_url: self.relay_url.clone(),
            duration_ms: 0, // Would measure in real implementation
        })
    }

    /// Contact the rendezvous mediator to get candidates
    async fn contact_mediator(
        &mut self,
        mediator_url: &str,
        client_id: &str,
    ) -> Result<(), String> {
        let client = Client::new();
        let mediator = Url::parse(mediator_url)
            .map_err(|e| format!("Invalid mediator URL: {}", e))?;

        let payload = serde_json::json!({
            "host_fingerprint": self.host_fingerprint,
            "client_id": client_id,
            "local_port": self.local_port
        });

        let response = client
            .post(&mediator.join("/connect").unwrap_or_default())
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Mediator connection failed: {}", e))?;

        if !response.status().is_success() {
            return Err("Mediator returned error status".to_string());
        }

        let mediator_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse mediator response: {}", e))?;

        // Extract candidates from response
        if let Some(candidates) = mediator_response.get("candidates").and_then(|c| c.as_array()) {
            for cand in candidates {
                if let Some(cand_obj) = cand.as_object() {
                    let candidate = Candidate {
                        component: cand_obj.get("component")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(1),
                        foundation: cand_obj.get("foundation")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        priority: cand_obj.get("priority")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0),
                        ip: cand_obj.get("ip")
                            .and_then(|v| v.as_str())
                            .unwrap_or("0.0.0.0")
                            .to_string(),
                        protocol: cand_obj.get("protocol")
                            .and_then(|v| v.as_str())
                            .unwrap_or("UDP")
                            .to_string(),
                        port: cand_obj.get("port")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u16,
                        candidate_type: cand_obj.get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("host")
                            .to_string(),
                        tcp_type: cand_obj.get("tcp_type")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    };
                    self.candidates.push(candidate);
                }
            }
        }

        // Extract relay URL if provided
        if let Some(relay) = mediator_response.get("relay_url").and_then(|v| v.as_str()) {
            self.relay_url = Some(relay.to_string());
        }

        // Extract client fingerprint
        if let Some(fp) = mediator_response.get("client_fingerprint").and_then(|v| v.as_str()) {
            self.client_fingerprint = fp.to_string();
        }

        // Extract client port
        if let Some(port) = mediator_response.get("client_port").and_then(|v| v.as_u64()) {
            self.client_port = port as u16;
        }

        // Update state
        self.state = HolePunchingStateEnum::Waiting;

        info!("Mediator contacted successfully");
        info!("Received {} candidates", self.candidates.len());
        info!("Client fingerprint: {}", self.client_fingerprint);
        info!("Client port: {}", self.client_port);

        Ok(())
    }

    /// Attempt P2P connection using hole punching
    async fn attempt_p2p(&mut self) -> Result<(), String> {
        info!("Attempting P2P connection via TCP hole punching");

        // Try each candidate pair
        for local_cand in &self.candidates {
            for remote_cand in &self.candidates {
                if local_cand.ip == self.client_fingerprint || remote_cand.ip == self.client_fingerprint {
                    continue; // Skip self
                }

                // Attempt TCP hole punching
                match self.try_tcp_hole_punch(local_cand, remote_cand).await {
                    Ok(true) => {
                        self.state = HolePunchingStateEnum::Connected;
                        info!("P2P connection established via hole punching!");
                        return Ok(());
                    }
                    Ok(false) => {
                        debug!("Hole punch attempt failed: local={}:{}, remote={}:{}",
                            local_cand.ip, local_cand.port, remote_cand.ip, remote_cand.port);
                    }
                    Err(e) => {
                        warn!("Hole punch error: {}", e);
                    }
                }
            }
        }

        // No direct P2P connection possible
        self.state = HolePunchingStateEnum::Failed;
        info!("P2P hole punching failed, will use relay");
        Ok(())
    }

    /// Try TCP hole punching between two candidates
    async fn try_tcp_hole_punch(
        &self,
        local: &Candidate,
        remote: &Candidate,
    ) -> Result<bool, String> {
        // Connect to remote candidate via TCP
        let remote_addr = format!("{}:{}", remote.ip, remote.port);
        let result = timeout(
            Duration::from_secs(5),
            TcpStream::connect(&remote_addr)
        ).await;

        match result {
            Ok(stream) => {
                info!("TCP connection established to {}:{}", remote.ip, remote.port);
                // TODO: Exchange SRTP keys, establish secure session
                Ok(true)
            }
            Err(e) => {
                debug!("TCP connection failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Set up relay connection as fallback
    async fn setup_relay(&mut self) -> Result<(), String> {
        info!("Setting up relay connection as fallback");

        if let Some(relay_url) = &self.relay_url {
            info!("Using relay: {}", relay_url);
            self.state = HolePunchingStateEnum::Relayed;
            Ok(())
        } else {
            error!("No relay URL available");
            Err("No relay URL available".to_string())
        }
    }
}

impl HolePunchingStateEnum {
    /// Return transport type string
    pub fn transport_type(&self) -> &'static str {
        match self {
            HolePunchingStateEnum::Initiated => "initiated",
            HolePunchingStateEnum::Waiting => "waiting",
            HolePunchingStateEnum::Checking => "checking",
            HolePunchingStateEnum::Connected => "p2p",
            HolePunchingStateEnum::Failed => "failed",
            HolePunchingStateEnum::Relayed => "relayed",
        }
    }
}

// Test module
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candidate_creation() {
        let cand = Candidate {
            component: 1,
            foundation: "test-foundation".to_string(),
            priority: 1000000,
            ip: "127.0.0.1".to_string(),
            protocol: "UDP".to_string(),
            port: 12345,
            candidate_type: "host".to_string(),
            tcp_type: None,
        };
        assert_eq!(cand.ip, "127.0.0.1");
        assert_eq!(cand.port, 12345);
    }

    #[test]
    fn test_state_transport_type() {
        assert_eq!(
            HolePunchingStateEnum::Connected.transport_type(),
            "p2p"
        );
        assert_eq!(
            HolePunchingStateEnum::Relayed.transport_type(),
            "relayed"
        );
    }
}