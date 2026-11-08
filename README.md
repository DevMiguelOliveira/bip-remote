# BIPRemote

Fundação técnica para uma plataforma profissional de suporte e acesso remoto a computadores Windows.

## Status

Esta etapa prepara o monorepo, os contratos iniciais, o modelo de dados e os esqueletos executáveis. Captura de tela, controle de entrada, execução remota de comandos e transferência de arquivos ainda não estão implementados.

## Estrutura

- `apps/admin`: desktop administrativo com Tauri 2, React e TypeScript.
- `agents/host`: host Windows e serviço privilegiado em Rust.
- `services/server`: monólito modular inicial em Go.
- `packages/ui`: componentes e tokens compartilhados do frontend.
- `protocols`: OpenAPI e schemas versionados para API, eventos e transporte remoto.
- `infrastructure`: banco de dados, desenvolvimento local e operação futura.
- `docs`: requisitos, arquitetura, segurança, decisões e operação.
- `tests`: espaços reservados para testes unitários, integração, E2E, segurança e carga.

## Requisitos locais

- Go 1.27+
- Rust 1.99+
- Node.js 26+
- pnpm 12+
- PostgreSQL 17+ para execução integrada

Docker é opcional. O compose em `infrastructure/development/docker-compose.yml` fornece somente o PostgreSQL local.

## Primeiros comandos

```powershell
Copy-Item .env.example .env
pnpm install --frozen-lockfile
go test ./services/server/...
cargo test --manifest-path agents/host/Cargo.toml
pnpm --filter @bipremote/admin build
```

Para subir o banco local:

```powershell
docker compose -f infrastructure/development/docker-compose.yml up -d postgres
```

Para validar a estrutura sem depender de serviços externos:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/validate-structure.ps1
```

## Princípios

1. O Host possui identidade criptográfica própria; um identificador legível nunca é credencial.
2. O serviço privilegiado é separado da UI e expõe somente IPC autenticado e mínimo.
3. Autorização é centralizada por organização, grupo e perfil de sessão auditável.
4. O backend começa como monólito modular; extração de serviços depende de evidência operacional.
5. Contratos versionados precedem a implementação das funcionalidades remotas.
