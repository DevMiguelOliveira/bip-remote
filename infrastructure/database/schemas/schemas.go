package schemas

import (
	"time"
)

// Session represents a remote session
type Session struct {
	ID              string
	OrganizationID  string
	HostID          string
	ClientID        string
	State           SessionState
	Transport       ConnectionTransport
	AuthorizedAt    *time.Time
	ApprovedBy      *string
	ApprovedAt      *time.Time
	StartedAt       *time.Time
	EndedAt         *time.Time
	DurationSeconds *int64
	CreatedAt       time.Time
	UpdatedAt       time.Time
}

// SessionState represents the state of a session
type SessionState string

const (
	SessionStatePending   SessionState = "pending"
	SessionStateApproved  SessionState = "approved"
	SessionStateActive    SessionState = "active"
	SessionStateEnded     SessionState = "ended"
	SessionStateFailed    SessionState = "failed"
	SessionStateRevoked   SessionState = "revoked"
)

// ConnectionTransport represents the transport method for a connection
type ConnectionTransport string

const (
	ConnectionTransportDirectP2P    ConnectionTransport = "direct_p2p"
	ConnectionTransportRelayed      ConnectionTransport = "relayed"
	ConnectionTransportTCPHolePunch ConnectionTransport = "tcp_hole_punch"
)

// EventType represents the type of event
type EventType string

const (
	EventTypeConnectionEstablished EventType = "connection.established"
	EventTypeConnectionFailed      EventType = "connection.failed"
	EventTypeSessionApproved       EventType = "session.approved"
	EventTypeSessionEnded          EventType = "session.ended"
	EventTypeStreamFrame           EventType = "stream.frame"
	EventTypeStreamAudio           EventType = "stream.audio"
	EventTypeControlInput          EventType = "control.input"
	EventTypeAuthChallenge         EventType = "auth.challenge"
	EventTypeAuthResponse          EventType = "auth.response"
)