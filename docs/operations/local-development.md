# Operação local

1. Copie `.env.example` para `.env`.
2. Suba PostgreSQL com o compose quando precisar de integração.
3. Aplique a migração inicial.
4. Inicie o backend com `go run ./services/server`.
5. Inicie o Admin com `pnpm --filter @bipremote/admin dev`.

O endpoint `/healthz` confirma processo vivo. O endpoint `/readyz` exige `BIPREMOTE_DATABASE_URL` configurada, mas ainda não realiza uma consulta de banco nesta fundação.
