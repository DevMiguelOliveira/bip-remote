# Segurança

Não reporte vulnerabilidades publicamente. Abra um canal privado com os mantenedores e inclua impacto, versão afetada, passos mínimos de reprodução e uma proposta de mitigação quando possível.

## Regras de segurança

- Não use identificadores de host como segredo.
- Não aceite comandos administrativos sem autenticação, autorização e auditoria.
- Não registre tokens, chaves privadas, cookies ou conteúdo de clipboard.
- Não desabilite validação TLS para testes de integração.
- Alterações no serviço privilegiado exigem revisão de segurança.

O modelo detalhado está em `docs/security/security-model.md`.
