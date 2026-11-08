# Banco de dados

As migrações SQL são a fonte de verdade do esquema inicial. A aplicação deve tratar o banco como infraestrutura externa e nunca depender de alterações manuais não versionadas.

## Migração local

Com PostgreSQL disponível, aplique `migrations/0001_initial.up.sql` usando a ferramenta de migração escolhida pela equipe. A etapa atual não fixa um runner de migrações para evitar uma dependência prematura; a decisão está registrada em `docs/decisions/ADR-0001-foundation.md`.
