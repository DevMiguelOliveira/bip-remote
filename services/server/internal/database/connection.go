package connection

import (
	"database/sql"
	"fmt"
	"log/slog"
	"strings"

	_ "github.com/jackc/pgx/v5/stdlib"
)

// Postgres represents a PostgreSQL connection pool
type Postgres struct {
	pool *sql.DB
}

// NewPostgres creates a new PostgreSQL connection pool
func NewPostgres(databaseURL string) (*Postgres, error) {
	// Clean up the URL - remove password from logging
	cleanURL := maskPassword(databaseURL)

	pool, err := sql.Open("pgx", databaseURL)
	if err != nil {
		return nil, fmt.Errorf("failed to open database connection: %w", err)
	}

	// Set connection pool settings
	pool.SetMaxOpenConns(20)
	pool.SetMaxIdleConns(5)
	pool.SetConnMaxLifetime(30 * 60 * 1e9) // 30 minutes

	// Test the connection
	if err := pool.Ping(); err != nil {
		return nil, fmt.Errorf("failed to ping database: %w", err)
	}

	slog.Info("database connection established", "url_masked", cleanURL)
	return &Postgres{pool: pool}, nil
}

// Close closes the database connection pool
func (p *Postgres) Close() error {
	return p.pool.Close()
}

// GetPool returns the underlying *sql.DB for use by other packages
func (p *Postgres) GetPool() *sql.DB {
	return p.pool
}

func maskPassword(url string) string {
	// Replace password with asterisks for logging safety
	// Format: postgresql://user:password@host/db -> postgresql://user:*****@host/db
	if idx := strings.Index(url, ":/"); idx != -1 {
		if idx > 4 { // There's a password portion
			end := strings.Index(url[idx:], "@")
			if end != -1 {
				inner := url[idx+2 : idx+2+end]
				inner = strings.Replace(inner, ":", "*****", 1)
				return url[:idx+2] + inner + url[idx+2+end:]
			}
		}
	}
	return url
}
