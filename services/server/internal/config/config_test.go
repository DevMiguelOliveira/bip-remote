package config

import "testing"

func TestLoadDefaults(t *testing.T) {
	t.Setenv("BIPREMOTE_ENVIRONMENT", "")
	t.Setenv("BIPREMOTE_HTTP_ADDR", "")
	t.Setenv("BIPREMOTE_LOG_LEVEL", "")
	t.Setenv("BIPREMOTE_DATABASE_URL", "")

	cfg, err := Load()
	if err != nil {
		t.Fatalf("Load() error = %v", err)
	}
	if cfg.Environment != "development" || cfg.HTTPAddress != "127.0.0.1:8080" {
		t.Fatalf("unexpected defaults: %+v", cfg)
	}
}

func TestLoadRejectsUnknownLogLevel(t *testing.T) {
	t.Setenv("BIPREMOTE_LOG_LEVEL", "trace")
	if _, err := Load(); err == nil {
		t.Fatal("Load() error = nil, want unsupported log level error")
	}
}
