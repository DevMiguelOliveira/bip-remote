package httpapi

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/DevMiguelOliveira/bip-remote/services/server/internal/config"
	"github.com/DevMiguelOliveira/bip-remote/infrastructure/database"
)

func newTestHandler(t *testing.T) http.Handler {
	pg, err := database.NewPostgres("postgresql://test:test@localhost:5432/test?sslmode=disable")
	if err != nil {
		t.Skip("PostgreSQL not available, skipping test")
	}
	return New(config.Config{}, pg, database.NewSessionStore(pg.GetPool()))
}

func TestHealth(t *testing.T) {
	handler := newTestHandler(t)
	request := httptest.NewRequest(http.MethodGet, "/healthz", nil)
	recorder := httptest.NewRecorder()

	handler.ServeHTTP(recorder, request)

	if recorder.Code != http.StatusOK {
		t.Fatalf("status = %d, want %d", recorder.Code, http.StatusOK)
	}
	if recorder.Header().Get("X-Request-ID") == "" {
		t.Fatal("X-Request-ID header is empty")
	}
}

func TestReadyRequiresDatabaseConfiguration(t *testing.T) {
	// Test without database - should return ServiceUnavailable
	handler := New(config.Config{DatabaseURL: ""}, nil, nil)
	request := httptest.NewRequest(http.MethodGet, "/readyz", nil)
	recorder := httptest.NewRecorder()

	handler.ServeHTTP(recorder, request)

	if recorder.Code != http.StatusServiceUnavailable {
		t.Fatalf("status = %d, want %d", recorder.Code, http.StatusServiceUnavailable)
	}
}