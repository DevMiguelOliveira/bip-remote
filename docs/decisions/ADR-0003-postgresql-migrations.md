# ADR-0003: Runner de migrations PostgreSQL

## Status

Aceito para a próxima integração de persistência. A dependência ainda não foi adicionada ao \`go.mod\` porque não havia PostgreSQL nem ferramenta de migration disponível durante a missão BIPREMOTE-004.

## Contexto

O projeto usa SQL versionado e um backend Go. O runner precisa aplicar migrations determinísticas, registrar a versão aplicada, impedir execução concorrente e sinalizar falha parcial sem depender de serviços pagos.

## Decisão

Adotar \`github.com/golang-migrate/migrate/v4\` como runner da aplicação e da automação de desenvolvimento.

As migrations permanecerão em \`infrastructure/database/migrations\` e serão executadas em ordem ascendente. O runner deverá:

- usar a tabela de controle própria do PostgreSQL;
- obter lock de migration no banco antes de aplicar alterações;
- marcar estado sujo quando uma migration falhar;
- executar cada migration em transação quando o SQL for compatível;
- impedir alteração de migration já aplicada;
- exigir uma nova migration para qualquer mudança posterior;
- manter rollback apenas como procedimento explícito e revisado.

O projeto não usará \`psql\` como runner de produção. \`psql\` será usado apenas pelos scripts de validação contra banco descartável.

## Alternativas

- \`pressly/goose\`: boa integração com Go e SQL, mas não oferece o mesmo fluxo de lock e estado sujo escolhido para esta fundação.
- \`Atlas\`: forte em drift detection e modelagem declarativa, mas adiciona uma ferramenta externa e uma mudança de fluxo maior neste estágio.
- \`psql\` manual: disponível como cliente PostgreSQL, porém não fornece controle de versão, concorrência ou estado de falha de migration.

## Consequências

- O backend terá uma biblioteca Go única para migrations e para o fluxo operacional.
- A escolha não elimina a necessidade de revisão de SQL, testes em banco descartável e backup antes de migrations destrutivas.
- Checksums de arquivo não serão tratados como controle nativo do runner; o CI deverá detectar alterações em migrations já publicadas antes da adoção da dependência.
- A adoção efetiva da dependência e o teste do runner permanecem pendentes até haver PostgreSQL e autorização para resolver dependências.
