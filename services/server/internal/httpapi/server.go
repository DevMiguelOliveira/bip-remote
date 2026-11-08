package httpapi

import (
	"encoding/json"
	"net/http"
	"time"

	"github.com/DevMiguelOliveira/bip-remote/services/server/internal/config"
)

type Server struct {
	config config.Config
}

func New(cfg config.Config) http.Handler {
	server := &Server{config: cfg}
	mux := http.NewServeMux()
	mux.HandleFunc("GET /healthz", server.health)
	mux.HandleFunc("GET /readyz", server.ready)
	mux.HandleFunc("GET /api/v1", server.apiRoot)
	mux.HandleFunc("GET /api/v1/", server.apiRoot)
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
