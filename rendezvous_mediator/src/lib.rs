//! Rendezvous Mediator for BIPRemote P2P Connection Coordination
//!
//! This module provides the core data structures and protocols for the
//! rendezvous mediator service that coordinates P2P connections between
//! BIPRemote hosts.

/// Host registration information published to the mediator
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostInfo {
    /// Unique cryptographic fingerprint (SHA-256 hex)
    pub fingerprint: String,
    /// Host capabilities (e.g., "video", "audio", "file_transfer")
    pub capabilities: Vec<String>,
    /// UDP/TCP port the host listens on for P2P connections
    pub listen_port: u16,
    /// Additional metadata (region, os, etc.)
    pub metadata: std::collections::HashMap<String, String>,
}

/// ICE candidate for P2P connection establishment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IceCandidate {
    /// Candidate line from SDP
    pub candidate: String,
    /// SDP media section index
    pub sdp_mline_index: Option<i32>,
    /// SDP media section name
    pub sdp_mid: Option<String>,
    /// Username fragment for ICE
    pub username_fragment: Option<String>,
    /// Password for ICE
    pub password: Option<String>,
}

/// Session state managed by the mediator
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SessionState {
    /// Session initiated, waiting for participants
    Initiated,
    /// Waiting for clients to connect
    WaitingForClients,
    /// Offer/answer exchange in progress
    ExchangeOffer,
    /// Answer received, session established
    ExchangeAnswer,
    /// Connection established via P2P or relay
    Connected,
    /// Connection failed, fall back to relay
    Failed,
    /// Using relay server as fallback
    Relayed,
}

/// Mediated session stored by the rendezvous service
#[derive(Clone, Debug)]
pub struct MediatedSession {
    /// Host's cryptographic fingerprint
    pub host_fingerprint: String,
    /// Client/peer identifier
    pub client_identifier: String,
    /// ICE candidates collected from host
    pub host_ice_candidates: Vec<IceCandidate>,
    /// ICE candidates collected from client
    pub client_ice_candidates: Vec<IceCandidate>,
    /// Session Description Protocol offer
    pub sdp_offer: Option<String>,
    /// Session Description Protocol answer
    pub sdp_answer: Option<String>,
    /// Current session state
    pub state: SessionState,
}

/// Error types for mediator operations
#[derive(Debug)]
pub enum MediatorError {
    SessionNotFound(String),
    SessionExists(String),
    InvalidFingerprint,
    InvalidSDP,
    ICEValidationFailed,
    Overloaded,
}

/// Result type for mediator operations
pub type MediatorResult<T> = Result<T, MediatorError>;

/// Create a new mediated session
pub fn create_session(
    host_fingerprint: String,
    client_identifier: String,
) -> MediatedSession {
    MediatedSession {
        host_fingerprint,
        client_identifier,
        host_ice_candidates: Vec::new(),
        client_ice_candidates: Vec::new(),
        sdp_offer: None,
        sdp_answer: None,
        state: SessionState::Initiated,
    }
}

/// Add an ICE candidate to a session
pub fn add_ice_candidate(
    session: &mut MediatedSession,
    candidate: IceCandidate,
    is_host: bool,
) {
    if is_host {
        session.host_ice_candidates.push(candidate);
    } else {
        session.client_ice_candidates.push(candidate);
    }
}

/// Set the SDP offer for a session
pub fn set_sdp_offer(session: &mut MediatedSession, offer: String) {
    session.sdp_offer = Some(offer);
    session.state = SessionState::ExchangeOffer;
}

/// Set the SDP answer for a session
pub fn set_sdp_answer(session: &mut MediatedSession, answer: String) {
    session.sdp_answer = Some(answer);
    session.state = SessionState::ExchangeAnswer;
}

/// Check if P2P connection is possible (have candidates from both sides)
pub fn can_attempt_p2p(session: &MediatedSession) -> bool {
    !session.host_ice_candidates.is_empty() && !session.client_ice_candidates.is_empty()
}

/// Transition session to connected state
pub fn mark_connected(session: &mut MediatedSession) {
    session.state = SessionState::Connected;
}

/// Mark session as failed (need relay fallback)
pub fn mark_failed(session: &mut MediatedSession) {
    session.state = SessionState::Failed;
}

/// Mark session as using relay
pub fn mark_relayed(session: &mut MediatedSession) {
    session.state = SessionState::Relayed;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_session() {
        let session = create_session("fp123".to_string(), "client1".to_string());
        assert_eq!(session.host_fingerprint, "fp123");
        assert_eq!(session.client_identifier, "client1");
        assert!(session.host_ice_candidates.is_empty());
        assert!(session.client_ice_candidates.is_empty());
        assert!(session.sdp_offer.is_none());
        assert!(session.sdp_answer.is_none());
        assert_eq!(session.state, SessionState::Initiated);
    }

    #[test]
    fn test_add_ice_candidate_host() {
        let mut session = create_session("fp123".to_string(), "client1".to_string());
        let candidate = IceCandidate {
            candidate: "candidate:UDP 1 typ relay".to_string(),
            ..Default::default()
        };
        add_ice_candidate(&mut session, candidate, true);
        assert_eq!(session.host_ice_candidates.len(), 1);
    }

    #[test]
    fn test_add_ice_candidate_client() {
        let mut session = create_session("fp123".to_string(), "client1".to_string());
        let candidate = IceCandidate {
            candidate: "candidate:UDP 1 typ relay".to_string(),
            ..Default::default()
        };
        add_ice_candidate(&mut session, candidate, false);
        assert_eq!(session.client_ice_candidates.len(), 1);
    }

    #[test]
    fn test_can_attempt_p2p() {
        let mut session = create_session("fp123".to_string(), "client1".to_string());
        assert!(!can_attempt_p2p(&session));

        let candidate = IceCandidate {
            candidate: "candidate:UDP 1 typ relay".to_string(),
            ..Default::default()
        };
        add_ice_candidate(&mut session, candidate, true);
        assert!(!can_attempt_p2p(&session)); // Need client candidate too

        add_ice_candidate(&mut session, candidate, false);
        assert!(can_attempt_p2p(&session));
    }

    #[test]
    fn test_session_state_transitions() {
        let mut session = create_session("fp123".to_string(), "client1".to_string());
        assert_eq!(session.state, SessionState::Initiated);

        set_sdp_offer(&mut session, "o=...".to_string());
        assert_eq!(session.state, SessionState::ExchangeOffer);

        set_sdp_answer(&mut session, "a=...".to_string());
        assert_eq!(session.state, SessionState::ExchangeAnswer);
    }
}