# Visão da arquitetura

O BIPRemote começa como um monólito modular em Go. O backend mantém o plano de controle: identidade, autorização, catálogo de hosts, autorização de sessões e auditoria. O Host Windows mantém a conexão autenticada e delega operações privilegiadas ao serviço separado.

```text
Admin (Tauri/React)
        |
   REST + WebSocket
        |
Backend Go modular ---- PostgreSQL
        |
  canal de controle autenticado
        |
Host Rust ---- IPC autenticado ---- Serviço Rust privilegiado
```

## Fronteiras

- `apps/admin` não acessa diretamente o banco nem executa operações privilegiadas.
- `services/server` não recebe credenciais privadas do Host.
- `agents/host/service` é a única fronteira para operações que exigem privilégio local.
- `protocols` é fonte de contrato; implementações devem ser compatíveis com versões publicadas.

## Escalabilidade inicial

A meta de projeto é 1.000 computadores cadastrados e 50 sessões simultâneas. O repositório não declara que a meta foi atingida sem benchmark; testes de carga serão adicionados em `tests/load`.
