package connection

import (
	"database/sql"
	"fmt"
	"log/slog"
	"time"

	"github.com/DevMiguelOliveira/bip-remote/infrastructure/database/schemas"
)

// SessionStore handles session-related database operations
type SessionStore struct {
	pool *sql.DB
}

// NewSessionStore creates a new SessionStore
func NewSessionStore(pool *sql.DB) *SessionStore {
	return &SessionStore{pool: pool}
}

// CreateSession creates a new remote session record
func (s *SessionStore) CreateSession(orgID, hostID, clientID string) (string, error) {
	var id string
	err := s.pool.QueryRow(`
		INSERT INTO sessions (organization_id, host_id, client_identifier, state, transport)
		VALUES ($1, $2, $3, 'pending', 'relayed')
		RETURNING id
	`, orgID, hostID, clientID).Scan(&id)

	if err != nil {
		return "", fmt.Errorf("failed to create session: %w", err)
	}

	slog.Info("session created", "session_id", id, "org_id", orgID, "host_id", hostID)
	return id, nil
}

// UpdateSessionState updates the state of a session
func (s *SessionStore) UpdateSessionState(sessionID string, state schemas.SessionState, approvedBy *string, approvedAt *time.Time) error {
	_, err := s.pool.Exec(`
		UPDATE sessions
		SET state = $1, authorized_at = $2, approved_by = $3, approved_at = $4
		WHERE id = $5
	`, state, approvedBy, approvedAt, sessionID)

	if err != nil {
		return fmt.Errorf("failed to update session state: %w", err)
	}

	return nil
}

// GetSession retrieves a session by ID
func (s *SessionStore) GetSession(sessionID string) (*schemas.Session, error) {
	var sess schemas.Session
	err := s.pool.QueryRow(`
		SELECT id, organization_id, host_id, client_identifier, state, transport,
               authorized_at, approved_by, approved_at, started_at, ended_at, duration_seconds,
               created_at, updated_at
		FROM sessions
		WHERE id = $1
	`, sessionID).Scan(
		&sess.ID,
		&sess.OrganizationID,
		&sess.HostID,
		&sess.ClientID,
		&sess.State,
		&sess.Transport,
		&sess.AuthorizedAt,
		&sess.ApprovedBy,
		&sess.ApprovedAt,
		&sess.StartedAt,
		&sess.EndedAt,
		&sess.DurationSeconds,
		&sess.CreatedAt,
		&sess.UpdatedAt,
	)

	if err != nil {
		if err == sql.ErrNoRows {
			return nil, fmt.Errorf("session not found: %s", sessionID)
		}
		return nil, fmt.Errorf("failed to retrieve session: %w", err)
	}

	return &sess, nil
}

// ListSessionsByOrganization lists sessions for an organization
func (s *SessionStore) ListSessionsByOrganization(orgID string) ([]schemas.Session, error) {
	rows, err := s.pool.Query(`
		SELECT id, organization_id, host_id, client_identifier, state, transport,
               authorized_at, approved_by, approved_at, started_at, ended_at, duration_seconds,
               created_at, updated_at
		FROM sessions
		WHERE organization_id = $1
		ORDER BY created_at DESC
	`, orgID)

	if err != nil {
		return nil, fmt.Errorf("failed to list sessions: %w", err)
	}
	defer rows.Close()

	var sessions []schemas.Session
	for rows.Next() {
		var sess schemas.Session
		if err := rows.Scan(
			&sess.ID,
			&sess.OrganizationID,
			&sess.HostID,
			&sess.ClientID,
			&sess.State,
			&sess.Transport,
			&sess.AuthorizedAt,
			&sess.ApprovedBy,
			&sess.ApprovedAt,
			&sess.StartedAt,
			&sess.EndedAt,
			&sess.DurationSeconds,
			&sess.CreatedAt,
			&sess.UpdatedAt,
		); err != nil {
			return nil, fmt.Errorf("failed to scan session: %w", err)
		}
		sessions = append(sessions, sess)
	}

	return sessions, nil
}

// EndSession ends a session record
func (s *SessionStore) EndSession(sessionID string, endedAt time.Time, durationSeconds int64) error {
	_, err := s.pool.Exec(`
		UPDATE sessions
		SET state = 'ended', ended_at = $1, duration_seconds = $2
		WHERE id = $3
	`, endedAt, durationSeconds, sessionID)

	if err != nil {
		return fmt.Errorf("failed to end session: %w", err)
	}

	return nil
}

// RecordSessionEvent records an audit event for a session
func (s *SessionStore) RecordSessionEvent(eventType schemas.EventType, sessionID, correlationID, payloadJSON, actorType string, actorID *string, resourceType *string, resourceID *string) error {
	_, err := s.pool.Exec(`
		INSERT INTO session_events (session_id, event_type, schema_version, correlation_id, payload, actor_type, actor_id, resource_type, resource_id)
		VALUES ($1, $2, 'v1', $3, $4::jsonb, $5, $6, $7, $8)
	`, sessionID, eventType, correlationID, payloadJSON, actorType, actorID, resourceType, resourceID)

	if err != nil {
		return fmt.Errorf("failed to record session event: %w", err)
	}

	return nil
}
