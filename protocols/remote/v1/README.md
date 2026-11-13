# BIPRemote Remote Protocol v1

Este diretório contém schemas JSON Schema versionados para os eventos e mensagens do protocolo remoto do BIPRemote.

## Versão Atual: v1

O protocolo v1 é a versão inicial e permanece imutável (não-breaking changes criam nova versão).

## Convencões

- Todos os eventos seguem o envelope versionado: `{ event_type, schema_version, correlation_id, payload, actor_type, actor_id, resource_type, resource_id }`
- `schema_version` deve ser `"v1"` para esta versão do protocolo
- `correlation_id` é um UUID para rastreamento distribuído
- `payload` contém dados específicos do `event_type`
- `actor_type` indica quem gerou o evento: `host`, `client` ou `system`
- `actor_id` é o UUID do ator (obrigatório quando `actor_type` não for `system`)

## Diretórios e Arquivos

```
v1/
  session_events.json  - Eventos de sessão (connection, session, auth)
  connection_events.json - Eventos de conexão (initiate, accepted, rejected, closed, error)
  stream_events.json   - Eventos de stream (start, frame, audio, stop, error)
  index.json           - Índice consolidado do protocolo
```

## Schemas Disponíveis

| Arquivo | Descrição |
|---|---|
| `session_events.json` | Eventos de controle de sessão e autenticação |
| `connection_events.json` | Eventos de estabelecimento e gerenciamento de conexão |
| `stream_events.json` | Eventos de transmissão de vídeo e áudio |

## Uso

Estes schemas são consumidos por:

1. **Backend Go** (`services/server`) - Validação de eventos entrantes via HTTP API
2. **Host Rust** (`agents/host`) - Geração e processamento de eventos
3. **Serviço Privilegiado** (`agents/host/service`) - Manipulação de eventos de sessão privilegiada
4. **Admin Console** (`apps/admin`) - Exibição e monitoramento de eventos em tempo real
5. **Banco de Dados** (`infrastructure/database`) - Armazenamento auditável de eventos

## Controle de Versão

Quando mudanças incompatíveis forem necessárias:
1. Crie um novo diretório `v2/`
2. Atualize o `schema_version` para `"v2"`
3. Migre consumidores existentes para a nova versão
4. Mantenha v1 ativo durante período de transição