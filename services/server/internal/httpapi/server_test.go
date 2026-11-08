package httpapi

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/DevMiguelOliveira/bip-remote/services/server/internal/config"
)

func TestHealth(t *testing.T) {
	handler := New(config.Config{})
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
	handler := New(config.Config{})
	request := httptest.NewRequest(http.MethodGet, "/readyz", nil)
	recorder := httptest.NewRecorder()

	handler.ServeHTTP(recorder, request)

	if recorder.Code != http.StatusServiceUnavailable {
		t.Fatalf("status = %d, want %d", recorder.Code, http.StatusServiceUnavailable)
	}
}
