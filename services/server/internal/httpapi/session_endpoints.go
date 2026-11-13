package httpapi

import (
	"encoding/json"
	"net/http"
)

// Session represents a remote session
type Session struct {
	ID           string `json:"id"`
	HostID       string `json:"host_id"`
	ClientID     string `json:"client_id"`
	State        string `json:"state"`
	Transport    string `json:"transport"`
	AuthorizedAt string `json:"authorized_at,omitempty"`
	ApprovedAt   string `json:"approved_at,omitempty"`
	StartedAt    string `json:"started_at,omitempty"`
	EndedAt      string `json:"ended_at,omitempty"`
	Duration     int64  `json:"duration_seconds,omitempty"`
}

// SessionApproval represents a session approval request
type SessionApproval struct {
	SessionID string `json:"session_id"`
	Approved  bool   `json:"approved"`
}

// SessionEnd represents a session end request
type SessionEnd struct {
	SessionID string `json:"session_id"`
}

// SessionListResponse represents the response for listing sessions
type SessionListResponse struct {
	Sessions []Session `json:"sessions"`
}

// HandleSessionList handles GET /api/v1/sessions
func (s *Server) HandleSessionList(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")

	// TODO: Query sessions from database based on operator/organization
	// For now, return empty list
	response := SessionListResponse{
		Sessions: []Session{},
	}

	writeJSON(w, http.StatusOK, response)
}

// HandleSessionApprove handles POST /api/v1/sessions/approve
func (s *Server) HandleSessionApprove(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")

	var approval SessionApproval
	if err := json.NewDecoder(r.Body).Decode(&approval); err != nil {
		http.Error(w, `{"error": "invalid_request"}`, http.StatusBadRequest)
		return
	}

	// TODO: Validate approval and update session state
	// - Verify operator permissions
	// - Update session state to "approved"
	// - Record approval timestamp
	// - Publish session.approved event

	response := map[string]bool{"approved": approval.Approved}
	writeJSON(w, http.StatusOK, response)
}

// HandleSessionEnd handles POST /api/v1/sessions/end
func (s *Server) HandleSessionEnd(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")

	var endReq SessionEnd
	if err := json.NewDecoder(r.Body).Decode(&endReq); err != nil {
		http.Error(w, `{"error": "invalid_request"}`, http.StatusBadRequest)
		return
	}

	// TODO: End the specified session
	// - Update session state to "ended"
	// - Record ended timestamp
	// - Calculate duration
	// - Publish session.ended event
	// - Cleanup resources

	response := map[string]bool{"ended": true}
	writeJSON(w, http.StatusOK, response)
}
