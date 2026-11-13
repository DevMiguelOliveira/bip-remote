# Banco de dados

As migrações SQL são a fonte de verdade do esquema. A aplicação deve tratar o banco como infraestrutura externa e nunca depender de alterações manuais não versionadas.

## Modelo atual

\`units\` é o nome físico da tabela que representa unidades organizacionais. Operadores são identidades globais e o pertencimento é controlado por \`organization_memberships\`. Hosts, credenciais, sessões, participantes e auditoria carregam \`organization_id\` ou uma relação composta que impede referências cruzadas.

A migration \`0002_multi_tenant_integrity\` adiciona:

- foreign keys compostas entre organização e entidade relacionada;
- unicidade case-insensitive dentro da organização;
- versões de credenciais públicas do Host;
- estado operacional separado do estado de confiança do Host;
- metadados de recurso, correlação e versão em \`audit_events\`;
- trigger append-only para UPDATE e DELETE de auditoria;
- RLS com \`FORCE ROW LEVEL SECURITY\` nas tabelas de dados e identidade;
- contexto transacional por \`app.current_organization_id\`.

## Contexto de tenant

Toda transação da aplicação deve definir o contexto antes de consultar ou alterar dados:

\`\`\`sql
BEGIN;
SELECT set_config('app.current_organization_id', '<organization-uuid>', true);
-- operações da transação
COMMIT;
\`\`\`

O terceiro argumento \`true\` torna a configuração local à transação e evita vazamento de contexto em conexões reutilizadas pelo pool. Sem contexto, as políticas RLS devem retornar zero linhas e rejeitar escritas.

O papel usado pela aplicação não pode possuir \`BYPASSRLS\`. A criação inicial de organizações e operadores deve ocorrer por fluxo de provisionamento controlado; esta missão não implementa autenticação nem provisionamento.

## Runner de migrations

O runner escolhido é \`github.com/golang-migrate/migrate/v4\`. A decisão e as alternativas estão registradas em \`docs/decisions/ADR-0003-postgresql-migrations.md\`. A dependência ainda não foi adicionada ao backend porque não havia PostgreSQL nem ferramenta disponível durante a missão e nenhuma instalação foi autorizada.

Até a integração do runner, \`psql\` pode ser usado somente para validação em banco descartável. Não use os scripts de validação contra produção.

## Validação local

Com PostgreSQL disponível, configure \`BIPREMOTE_TEST_DATABASE_URL\` para um banco descartável e execute:

\`\`\`powershell
.\\scripts\\validate-database.ps1
\`\`\`

O script aplica todas as migrations em ordem e executa \`tests/database/tenant-isolation.sql\`. A validação cobre visibilidade por tenant, contexto ausente, foreign keys compostas e rejeição de associações cruzadas. Ela ainda precisa ser executada em PostgreSQL real.

## Rollback e recuperação

Rollback deve ser executado somente em banco descartável ou após backup e revisão explícita. A migration down não deve ser usada para apagar dados de produção. Antes de alterar schema em ambiente persistente:

1. confirmar backup restaurável;
2. aplicar em staging;
3. verificar versão e integridade;
4. observar erros e locks;
5. manter procedimento de recuperação documentado.


## Hardening de Segurança e Roles (PostgreSQL)

O BIPRemote utiliza uma arquitetura de privilégio mínimo baseada em papéis de grupo (*NOLOGIN*):

1. **`bip_migrator`**: Papel exclusivo para aplicação de migrações DDL e administração do esquema nos schemas `public` e `app`.
2. **`bip_app`**: Papel de *runtime* da aplicação. Possui privilégios DML restritos (`SELECT, INSERT, UPDATE, DELETE`) nas tabelas de negócio e apenas `SELECT, INSERT` na tabela de auditoria (`audit_events`). Não possui `BYPASSRLS`, permissão de DDL, `TRUNCATE` ou administração de esquema.

### Estado de Implementação e Validação

- **PostgreSQL 18.6**: Instalado no ambiente local de desenvolvimento.
- **Migrations 0001 e 0002**: Executadas e validadas no banco descartável `bipremote_validation`.
- **Suíte de Isolamento Multi-tenant (`tests/database/tenant-isolation.sql`)**: Executada com sucesso sob role não-superusuária em `bipremote_validation`.
- **Bootstrap (`infrastructure/database/bootstrap/admin-bootstrap.sql`)**: Validado estaticamente. Cria e valida os atributos de `bip_migrator` e `bip_app` de forma idempotente.
- **Migration 0003 (`0003_security_hardening`)**: Criada e validada estaticamente. **Ainda não foi aplicada ao banco de dados real/validação.**

### Bootstrap Administrativo

O bootstrap de roles é um procedimento de infraestrutura de nível de cluster e **não** roda como migration da aplicação. Um administrador do PostgreSQL deve executar:

```bash
psql -h 127.0.0.1 -U postgres -d bipremote_validation -f infrastructure/database/bootstrap/admin-bootstrap.sql
```

As identidades de conexão (`LOGIN`) devem ser provisionadas separadamente pela infraestrutura e associadas às roles de grupo (`GRANT bip_app TO app_user;`). O contexto GUC (`app.current_organization_id`) estabelece o escopo de tenant do RLS para a transação e não substitui a autenticação/autorização da aplicação no backend.
