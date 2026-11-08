# ADR-0001: Fundação modular e contratos primeiro

## Contexto

O produto precisa suportar Windows, um desktop administrativo, um backend central e sessões remotas sem criar dependência paga ou microsserviços prematuros.

## Decisão

- Usar Go para o monólito modular do backend.
- Usar Rust para Host e serviço privilegiado.
- Usar Tauri 2, React e TypeScript para o Admin.
- Usar PostgreSQL como banco relacional inicial.
- Versionar OpenAPI, schemas de eventos e migrações SQL.
- Manter desenvolvimento local com PostgreSQL opcional via Docker Compose.

## Alternativas avaliadas

- **Microsserviços desde o início:** maior isolamento, porém custo operacional e complexidade sem evidência de necessidade.
- **Backend em Node.js:** ecossistema frontend compartilhado, porém Go oferece um runtime enxuto e fronteira clara para o plano de controle.
- **Serviço Windows em C#:** integração Windows madura, porém Rust mantém uma linguagem comum com o Host e reduz a superfície de runtime.
- **SQLite no backend:** custo local menor, porém PostgreSQL representa melhor concorrência, auditoria e evolução corporativa.
- **Runner de migração dedicado agora:** ergonomia maior, porém a escolha pode ser feita junto com a camada de persistência real.

## Consequências

O primeiro incremento deve investir em contratos, autenticação e testes de integração. A extração de componentes ou a escolha de provedores será guiada por benchmarks e requisitos operacionais, não por antecipação.
