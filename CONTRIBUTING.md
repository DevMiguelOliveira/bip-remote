# Contribuição

## Fluxo

1. Leia `docs/architecture/overview.md`, `docs/security/security-model.md` e a ADR relevante.
2. Trabalhe dentro do escopo do módulo alterado.
3. Não adicione credenciais, tokens, dumps ou dados reais.
4. Atualize contratos e documentação junto com mudanças de comportamento.
5. Execute as validações específicas e registre falhas conhecidas no pull request.

## Divisão de responsabilidades

- Backend: `services/server` e `protocols/api`.
- Banco: `infrastructure/database`.
- Admin: `apps/admin` e `packages/ui`.
- Host: `agents/host` e `protocols/remote`.
- QA/Security: `docs/security`, `tests/security` e workflows.

Evite alterações simultâneas no mesmo contrato ou migração. Quando uma mudança cruzar módulos, registre a dependência em `docs/decisions`.

## Critérios mínimos

- Formatação e lint sem novos erros.
- Testes automatizados para comportamento novo.
- Compatibilidade com os schemas versionados.
- Logs sem segredos ou dados pessoais desnecessários.
