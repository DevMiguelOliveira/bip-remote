# Governança por módulo

| Módulo | Responsável primário | Pode alterar | Deve consultar |
| --- | --- | --- | --- |
| Backend | Backend | `services/server`, endpoints OpenAPI | banco, segurança |
| Banco | Banco | migrações SQL, índices, retenção | backend, operação |
| Admin | Desktop | `apps/admin`, `packages/ui` | OpenAPI, IPC |
| Host | Host Windows | `agents/host`, `protocols/remote` | segurança, backend |
| QA/Security | Qualidade | `tests`, workflows e políticas | todos os contratos |

Alterações que cruzem duas linhas devem registrar uma decisão em `docs/decisions` antes da implementação.
