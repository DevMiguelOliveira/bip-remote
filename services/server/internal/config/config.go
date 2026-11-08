package config

import (
	"fmt"
	"log/slog"
	"os"
)

type Config struct {
	Environment    string
	HTTPAddress    string
	DatabaseURL    string
	LogLevel       slog.Level
	AllowedOrigins string
}

func Load() (Config, error) {
	cfg := Config{
		Environment:    valueOrDefault("BIPREMOTE_ENVIRONMENT", "development"),
		HTTPAddress:    valueOrDefault("BIPREMOTE_HTTP_ADDR", "127.0.0.1:8080"),
		DatabaseURL:    os.Getenv("BIPREMOTE_DATABASE_URL"),
		AllowedOrigins: valueOrDefault("BIPREMOTE_ALLOWED_ORIGINS", "http://localhost:1420"),
	}

	logLevel, err := parseLogLevel(valueOrDefault("BIPREMOTE_LOG_LEVEL", "info"))
	if err != nil {
		return Config{}, err
	}
	cfg.LogLevel = logLevel
	return cfg, nil
}

func valueOrDefault(key string, fallback string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return fallback
}

func parseLogLevel(value string) (slog.Level, error) {
	switch value {
	case "debug":
		return slog.LevelDebug, nil
	case "info":
		return slog.LevelInfo, nil
	case "warn", "warning":
		return slog.LevelWarn, nil
	case "error":
		return slog.LevelError, nil
	default:
		return 0, fmt.Errorf("unsupported log level %q", value)
	}
}
