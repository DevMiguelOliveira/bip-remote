# Protocolo remoto

Este diretório reserva os contratos para a comunicação entre Host, serviço privilegiado e backend.

## Regras iniciais

- O Host autentica com identidade criptográfica individual, não com um código compartilhado.
- O serviço privilegiado não expõe uma porta de escuta pública.
- A UI conversa com o serviço por IPC autenticado e autorizado.
- O plano de controle e o canal de mídia permanecem separados.
- Criptografia ponta a ponta, rotação e revogação devem ser especificadas antes da implementação.

## Estrutura do Protocolo v1

```
v1/
  session_events.json  - Eventos de sessão (connection, session, auth)
  connection_events.json - Eventos de conexão (initiate, accepted, rejected, closed, error)
  stream_events.json   - Eventos de stream (start, frame, audio, stop, error)
  README.md            - Esta visão geral
  index.json           - Índice consolidado do protocolo
```

## Eventos Versionados

Todos os eventos seguem o envelope versionado v1 com os campos obrigatórios:
- `event_type`: Tipo do evento (ver schemas individuais)
- `schema_version`: Deve ser `"v1"`
- `correlation_id`: UUID para rastreamento distribuído
- `payload`: Dados específicos do event_type
- `actor_type`: `host`, `client` ou `system`

## Uso pelos Componentes

- **Backend Go** (`services/server`): Validação via OpenAPI/JSON Schema
- **Host Rust** (`agents/host`): Geração e processamento de eventos
- **Serviço Privilegiado** (`agents/host/service`): Manipulação de eventos privilegiados
- **Admin Console** (`apps/admin`): Monitoramento e exibição em tempo real
- **Banco de Dados** (`infrastructure/database`): Armazenamento auditável