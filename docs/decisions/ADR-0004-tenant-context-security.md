# ADR-0004: Contexto de tenant e segurança do isolamento multi-tenant

## Status

Proposta para revisão e aprovação expressa. Nenhuma decisão de segurança está escondida em suposições; todas as recomendações pendentes estão explicitamente marcadas.

## Contexto

O BIPRemote adota PostgreSQL com Row Level Security (RLS) e `FORCE ROW LEVEL SECURITY` em nove tabelas de dados e identidade. O isolamento entre organizações depende de uma variável de sessão customizada (`app.current_organization_id`) lida pela função `app.current_organization_id()` e comparada nas policies RLS contra a coluna `organization_id` de cada linha.

O contexto atual pode ser alterado por qualquer sessão que consiga executar SQL com a role da aplicação. O uso de `set_config(..., true)` reduz o vazamento entre transações em pool de conexões, mas não autentica nem autoriza o tenant: a variável de sessão, por si só, não é uma fronteira de segurança.

O backend é um monólito modular em Go que se conecta ao PostgreSQL via pool de conexões. As migrations `0001` e `0002` criam o schema, as funções, as constraints, as políticas RLS e o trigger append-only de `audit_events`, mas não criam papéis de banco, não concedem privilégios e não definem quem pode alterar o contexto de organização.

Esta missão é de análise e decisão arquitetural. Não implementa autenticação, não modifica migrations, não cria roles no banco e não realiza commits.

## Problema

Garantir que consultas e escritas do backend nunca escapem do escopo da organização autenticada, mesmo diante de:
- reutilização de conexão em pool;
- tentativa de troca arbitrária de `app.current_organization_id`;
- SQL injection;
- papel da aplicação com privilégios excessivos (`BYPASSRLS`, `SUPERUSER`, propriedade de tabelas);
- operador pertencente a múltiplas organizações;
- revogação de membership durante sessão ativa.

## Requisitos de segurança

1. **Isolamento estrito:** uma organização nunca enxerga nem escreve linhas de outra organização.
2. **Contexto confiável:** o valor de `app.current_organization_id` só pode ser definido por fluxo autorizado, nunca arbitrariamente pela aplicação.
3. **Falha fechada (fail closed):** contexto ausente, vazio ou inválido resulta em zero linhas visíveis e escritas rejeitadas.
4. **Nenhum vazamento por pool:** conexão reutilizada não carrega contexto de outra organização entre transações.
5. **Sem privilégios de bypass:** o papel de execução da aplicação não possui `BYPASSRLS`, `SUPERUSER`, propriedade de tabelas nem capacidade de definir a GUC livremente.
6. **Auditabilidade:** alterações de membership e tentativas de alteração de contexto devem ser auditáveis.

## Alternativas consideradas

### A. GUC customizada controlada pelo backend

O backend autentica o operador, verifica a membership na organização, inicia uma transação e define `set_config('app.current_organization_id', '<uuid>', true)` antes de executar consultas sob RLS.

Ameaças:
- SQL injection poderia executar `SET app.current_organization_id = '<outro-uuid>'` se o papel da aplicação tiver privilégio de `SET` na GUC.
- Pool de conexões: se o contexto for definido em nível de sessão (`false`), vazará entre transações.
- Comprometimento do backend permite definir qualquer contexto.

Mitigações:
- Usar `set_config(..., true)` (escopo de transação).
- Revogar privilégio de `SET` na GUC para o papel da aplicação, quando o PostgreSQL permitir.
- Usar prepared statements e parâmetros para evitar SQL injection.
- Criar uma função `SECURITY DEFINER` no schema `app`, owned por role privilegiada, que valide a membership antes de definir o contexto, e conceder `EXECUTE` apenas a essa função para `bipremote_app`.

### B. Contexto autenticado por função PostgreSQL (`SECURITY DEFINER`)

Uma função `app.set_current_organization(organization_uuid, operator_uuid)` definida com `SECURITY DEFINER`, owned por `bipremote_migrations` ou papel de tabela, que:
- verifica se existe `organization_memberships (organization_id, operator_id)` para os argumentos;
- apenas então chama `set_config('app.current_organization_id', organization_uuid, true)`;
- lança exceção se a membership não existir.

A aplicação chama essa função no início de cada transação em vez de `SET` direto.

Limitações:
- Quando várias organizações compartilham a mesma credencial de banco (único papel `bipremote_app`), a função não consegue distinguir "quem está chamando" além do que a aplicação passar como parâmetro; a confiança ainda reside na aplicação para passar o `operator_uuid` correto.
- `search_path` inseguro permite shadowing da função; deve ser qualificada explicitamente (`app.set_current_organization`) e o `search_path` da função deve ser fixado.
- `SECURITY DEFINER` roda com privilégios do owner; se o owner for super ou proprietário de tabelas, escopo de privilégios deve ser minimizado.

### C. Identidade de banco por operador ou organização

Cada organização (ou operador) tem credenciais de banco distintas. O próprio papel de conexão carrega a identidade e o RLS pode ser baseado em `current_user` ou em uma função que mapeia o role para organização.

Ameaças e custos:
- Pool de conexões precisa ser particionado por organização/operador, o que é operacionalmente complexo e não escala bem para milhares de hosts e múltiplas sessões.
- Rotação e revogação de credenciais por organização aumentam o overhead operacional.
- Não elimina a necessidade de validar membership: mesmo com credencial distinta, o banco precisa saber quais organizações esse role pode acessar.

Não recomendada para o estágio atual pelo custo operacional e complexidade de pool.

### D. Modelo intermediário (híbrido recomendado)

Combina:
- **A** para o fluxo transacional padrão (backend define contexto via função validada);
- **B** como mecanismo de controle no banco (função `SECURITY DEFINER` que valida membership antes de definir o contexto);
- Controle de privilégios no papel `bipremote_app` (sem `BYPASSRLS`, sem `SUPERUSER`, não proprietário das tabelas);
- Uso obrigatório de `set_config(..., true)` dentro de transações explícitas;
- Prepared statements e parâmetros no Go para mitigar SQL injection.

## Matriz comparativa

| Critério | A (GUC backend) | B (Função banco) | C (Credencial por org) | D (Híbrido recomendado) |
|---|---|---|---|---|
| Isolamento entre organizações | Depende da aplicação | Depende da aplicação passar dados corretos | Forte, mas custoso | Forte com controle em camadas |
| Resiste a troca arbitrária de GUC | Não, sem revogar SET | Sim, se aplicação não tiver SET direto | Sim | Sim |
| Resiste a vazamento de pool | Parcial (precisa `true`) | Parcial (precisa `true`) | Sim (conexão dedicada) | Sim |
| Complexidade operacional | Baixa | Média | Alta | Média |
| Escala para 1.000 Hosts | Sim | Sim | Limitada por pool | Sim |
| Requer PostgreSQL real para validar | Sim | Sim | Sim | Sim |

## Decisão recomendada

Adotar o **modelo híbrido (D)**:

1. O backend autentica o operador (fora do escopo desta missão) e resolve sua membership na organização-alvo.
2. No início de cada transação, o backend chama a função `app.set_current_organization(organization_id, operator_id)` (`SECURITY DEFINER`, owned por papel privilegiado), que valida a membership em `organization_memberships` e só então define `set_config('app.current_organization_id', organization_id, true)`.
3. Todas as consultas DML ocorrem dentro da mesma transação, sob RLS.
4. Ao final (commit ou rollback), o contexto some por ser de escopo transacional (`true`).
5. O papel `bipremote_app` não tem `BYPASSRLS`, não tem `SUPERUSER`, não é proprietário das tabelas e não deve ter privilégio de `SET` direto na GUC.
6. Funções `SECURITY DEFINER` usam `search_path` fixo e qualificação explícita de objetos (`app.`).
7. Operações administrativas (revogação de membership, auditoria) são explicitamente documentadas e requerem papel separado.

Decisões já aprovadas (não alterar):
- RLS com `FORCE ROW LEVEL SECURITY` nas nove tabelas (migration `0002`).
- Função `app.current_organization_id()` baseada em `current_setting`.
- Operador como identidade global; `organization_memberships` como vínculo.

Recomendações pendentes de aprovação:
- Criação da função `app.set_current_organization` (requer nova migration ou alteração de `0002`, fora do escopo atual).
- Definição dos papéis `bipremote_migrations`, `bipremote_app`, `bipremote_admin` e seus privilégios (não executar `CREATE ROLE` nesta missão).
- Revogação de privilégio de `SET` na GUC para `bipremote_app`.

## Consequências positivas

- Isolamento imposto no banco, não apenas no código do backend.
- Falha fechada quando o contexto está ausente ou inválido.
- Nenhum vazamento de contexto entre transações reutilizadas no pool.
- Superfície de ataque reduzida: a aplicação não pode definir a GUC arbitrariamente.
- Compatível com migrations existentes (`0001` e `0002` não precisam ser alteradas).

## Consequências negativas

- Adiciona uma chamada de função no início de cada transação (overhead mínimo).
- A segurança ainda depende do backend chamar a função de contexto em toda transação; uma transação esquecida fica sem contexto e, portanto, sem dados visíveis (fail closed, não fail open).
- Quando várias organizações compartilham o mesmo papel de banco, a função `SECURITY DEFINER` confia no `operator_uuid` informado pela aplicação; comprometimento do backend permite se passar por outro operador dentro das organizações às quais ele tem membership.
- Requer PostgreSQL real para validação das políticas RLS e da função de contexto.

## Ameaças residuais

| Ameaça | Vetor | Impacto | Mitigação | Risco residual |
|---|---|---|---|---|
| Troca arbitrária de `app.current_organization_id` | SQL injection ou `SET` direto pelo papel da aplicação | Isolamento quebrado | Revogar `SET`; usar função `SECURITY DEFINER` validada | Médio (depende do controle de privilégios) |
| Vazamento por pool de conexões | `set_config(..., false)` ou contexto não limpo | Transação vê dados de outra org | Obrigar `set_config(..., true)`; reset no checkout da conexão | Baixo/Médio |
| Comprometimento do backend | Atacante controla o código Go | Define qualquer contexto para orgs onde o operador tem membership | Não elimina; limita ao escopo de membership do operador | Alto (aceito architecturalmente) |
| Operador em múltiplas organizações | Backend define contexto para org A, depois org B | Operador acessa ambas em transações separadas (esperado) | RLS permite apenas a org do contexto atual | Baixo (by design) |
| Revogação de membership durante sessão ativa | Membership removida, mas contexto ainda válido na transação | Transação em andamento completa com acesso antigo | Transações curtas; revalidar membership em cada transação nova | Médio |
| Role com `BYPASSRLS` ou `SUPERUSER` | Papel da aplicação com privilégios excessivos | RLS ignorado completamente | Proibir `BYPASSRLS`/`SUPERUSER` em `bipremote_app` | Baixo (se aplicado) |
| Proprietário de tabelas | Papel da aplicação é owner das tabelas | Owner ignora RLS por padrão | `FORCE ROW LEVEL SECURITY` + não tornar app proprietário | Baixo (se aplicado) |
| Funções `SECURITY DEFINER` inseguras | `search_path` shadowing, owner excessivo | Execução com privilégios não intencionados | `search_path` fixo, qualificação explícita, owner não-super | Baixo/Médio |
| Acesso direto ao banco | Usuário com credencial direta acessa PostgreSQL | Pode definir GUC se tiver privilégio | Controle de privilégios de `SET` e roles separados | Médio |
| SQL injection | Parâmetros não sanitizados em consultas Go | Leitura/escrita fora de escopo ou alteração de GUC | Prepared statements, parâmetros, ORM/query builder seguro | Baixo |

## Plano de validação

Os seguintes testes exigem PostgreSQL real (não usar SQLite ou mocks):

1. Tenant A não acessa registros do Tenant B (SELECT, INSERT, UPDATE, DELETE).
2. Tentativa de `SET app.current_organization_id` pelo papel da aplicação é rejeitada ou não afeta o isolamento.
3. Contexto ausente resulta em zero linhas.
4. Contexto inválido (não-UUID) falha com erro (fail closed).
5. Reutilização de conexão: contexto não persiste entre transações (`set_config(..., true)`).
6. SQL injection em parâmetros e consultas não quebra o isolamento.
7. Membership revogada: nova transação não enxerga a organização.
8. Papel de execução sem `BYPASSRLS` não ignora RLS.
9. Acesso de proprietário e role administrativa documentado e restrito.
10. Funções `SECURITY DEFINER` não sofrem shadowing via `search_path`.
11. Operações concorrentes de diferentes organizações isoladas corretamente.

## Questões ainda não decididas

1. Se o privilégio de `SET` na GUC customizada pode ser revogado para o papel da aplicação e se o PostgreSQL suporta isso sem quebrar a função `SECURITY DEFINER` (validar semântica documentada do PostgreSQL).
2. Se a função `app.set_current_organization` será adicionada em uma nova migration ou integrada à `0002` antes da adoção em produção.
3. Qual papel será o owner das tabelas (`bipremote_migrations` ou role NOLOGIN separada).
4. Se haverá um papel `bipremote_admin` e quais operações ele poderá executar.
5. Como o backend Go vai provedor o `operator_uuid` para a função de contexto sem implementar autenticação nesta fase.

## ADRs relacionados

- ADR-0001: Fundação modular e contratos primeiro.
- ADR-0003: Runner de migrations PostgreSQL.

## Aprovação

Esta recomendação está pronta para revisão. Não iniciar implementação automática após produzir este ADR. Aguardar revisão e aprovação expressa.
