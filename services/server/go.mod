module github.com/DevMiguelOliveira/bip-remote/services/server

go 1.27.0

require (
	github.com/jackc/pgx/v5 v5.4.0
	github.com/google/uuid v1.6.0
	github.com/joho/godotenv v1.4.0
)

require (
	go.mozilla.org/sqlite/v1 v1.28.6 // indirect
)

exclude (
	go.mozilla.org/sqlite/v1 v1.29.0 // replaced by v1.28.6
)