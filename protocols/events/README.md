# Eventos

Eventos são envelopes versionados, imutáveis e auditáveis. O transporte inicial previsto é WebSocket autenticado por sessão; o conteúdo do evento não substitui o registro de auditoria persistido no backend.

O campo `schemaVersion` é obrigatório. Mudanças incompatíveis criam uma nova versão do schema, em vez de alterar silenciosamente a versão existente.
