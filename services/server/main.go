package main

import (
	"context"
	"errors"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/DevMiguelOliveira/bip-remote/services/server/internal/config"
	"github.com/DevMiguelOliveira/bip-remote/services/server/internal/httpapi"
	"github.com/DevMiguelOliveira/bip-remote/infrastructure/database"
)

func main() {
	cfg, err := config.Load()
	if err != nil {
		slog.Error("invalid configuration", "error", err)
		os.Exit(1)
	}

	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: cfg.LogLevel}))
	slog.SetDefault(logger)

	// Initialize database connection
	pg, err := database.NewPostgres(cfg.DatabaseURL)
	if err != nil {
		slog.Error("failed to initialize database", "error", err)
		os.Exit(1)
	}
	defer pg.Close()

	// Initialize session store
	sessionStore := database.NewSessionStore(pg)

	logger.Info("database initialized", "url_masked", maskDatabaseURL(cfg.DatabaseURL))

	server := &http.Server{
		Addr:              cfg.HTTPAddress,
		Handler:           httpapi.New(cfg, pg, sessionStore),
		ReadHeaderTimeout: 5 * time.Second,
		ReadTimeout:       15 * time.Second,
		WriteTimeout:      15 * time.Second,
		IdleTimeout:       60 * time.Second,
	}

	serverErrors := make(chan error, 1)
	go func() {
		logger.Info("server started", "address", cfg.HTTPAddress, "environment", cfg.Environment)
		serverErrors <- server.ListenAndServe()
	}()

	shutdownContext, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	select {
	case err := <-serverErrors:
		if !errors.Is(err, http.ErrServerClosed) {
			logger.Error("server stopped unexpectedly", "error", err)
			os.Exit(1)
		}
	case <-shutdownContext.Done():
		shutdownContext, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		if err := server.Shutdown(shutdownContext); err != nil {
			logger.Error("graceful shutdown failed", "error", err)
			os.Exit(1)
		}
	}
}

func maskDatabaseURL(url string) string {
	// Mask the password in the database URL for logging
	if len(url) > 20 {
		return url[:20] + "*****" + url[len(url)-10:]
	}
	return url
}