# Eventos

Eventos são envelopes versionados e auditáveis. O transporte inicial previsto é WebSocket autenticado por sessão; o conteúdo do evento não substitui o registro de auditoria persistido no backend.

O envelope v1 permanece imutável. O envelope v2 inclui \`organizationId\` para manter o escopo de tenant explícito durante transporte e roteamento. Mudanças incompatíveis criam uma nova versão do schema, em vez de alterar silenciosamente a versão existente.

O payload de auditoria v2 usa \`action\` como equivalente contratual de \`audit_events.event_type\`, inclui \`correlationId\` e \`schemaVersion\`, e pode apontar para um recurso por \`resourceType\` e \`resourceId\`. \`actorType = system\` não possui \`actorId\`.
