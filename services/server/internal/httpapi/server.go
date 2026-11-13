package httpapi

import (
	"encoding/json"
	"net/http"
	"time"

	"github.com/DevMiguelOliveira/bip-remote/services/server/internal/config"
	"github.com/DevMiguelOliveira/bip-remote/infrastructure/database"
)

type Server struct {
	config       config.Config
	db           *database.Postgres
	sessionStore *database.SessionStore
}

// New creates a new HTTP API server
func New(cfg config.Config, db *database.Postgres, ss *database.SessionStore) http.Handler {
	server := &Server{
		config:       cfg,
		db:           db,
		sessionStore: ss,
	}
	mux := http.NewServeMux()
	mux.HandleFunc("GET /healthz", server.health)
	mux.HandleFunc("GET /readyz", server.ready)
	mux.HandleFunc("GET /api/v1", server.apiRoot)
	mux.HandleFunc("GET /api/v1/sessions", server.HandleSessionList)
	mux.HandleFunc("POST /api/v1/sessions/approve", server.HandleSessionApprove)
	mux.HandleFunc("POST /api/v1/sessions/end", server.HandleSessionEnd)
	return requestID(mux)
}

func (s *Server) health(writer http.ResponseWriter, _ *http.Request) {
	writeJSON(writer, http.StatusOK, map[string]string{"status": "ok"})
}

func (s *Server) ready(writer http.ResponseWriter, _ *http.Request) {
	if s.config.DatabaseURL == "" {
		writeJSON(writer, http.StatusServiceUnavailable, map[string]string{"status": "not_ready", "reason": "database_not_configured"})
		return
	}
	writeJSON(writer, http.StatusOK, map[string]string{"status": "ready"})
}

func (s *Server) apiRoot(writer http.ResponseWriter, _ *http.Request) {
	writeJSON(writer, http.StatusNotImplemented, map[string]string{"error": "api_not_implemented"})
}

func requestID(next http.Handler) http.Handler {
	return http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		requestID := request.Header.Get("X-Request-ID")
		if requestID == "" {
			requestID = time.Now().UTC().Format("20060102150405.000000000")
		}
		writer.Header().Set("X-Request-ID", requestID)
		next.ServeHTTP(writer, request)
	})
}

func writeJSON(writer http.ResponseWriter, status int, value any) {
	writer.Header().Set("Content-Type", "application/json")
	writer.WriteHeader(status)
	_ = json.NewEncoder(writer).Encode(value)
}
