# Modelo de dados multi-tenant

## Regra de isolamento

Todas as entidades que pertencem a uma organização carregam \`organization_id\` diretamente ou por uma relação composta. As foreign keys compostas impedem que uma linha de uma organização referencie uma unidade, Host, sessão ou operador-membro de outra organização.

\`operators\` representa a identidade global do operador. O pertencimento é controlado por \`organization_memberships\`; um operador só é visível para o contexto da organização quando possui membership nessa organização.

O contexto do tenant é transacional:

\`\`\`sql
BEGIN;
SELECT set_config('app.current_organization_id', '<organization-uuid>', true);
-- consultas e mutações da transação
COMMIT;
\`\`\`

Sem esse contexto, as políticas RLS não retornam linhas e rejeitam escritas. O serviço não deve usar um papel com \`BYPASSRLS\`; \`FORCE ROW LEVEL SECURITY\` também impede o bypass implícito do proprietário das tabelas.

## Diagrama

\`\`\`mermaid
erDiagram
    organizations ||--o{ units : contains
    organizations ||--o{ organization_memberships : grants
    operators ||--o{ organization_memberships : joins
    organizations ||--o{ hosts : owns
    units ||--o{ hosts : groups
    hosts ||--o{ host_credentials : rotates
    organizations ||--o{ sessions : scopes
    hosts ||--o{ sessions : serves
    organization_memberships ||--o{ sessions : requests
    sessions ||--o{ session_participants : includes
    organization_memberships ||--o{ session_participants : participates
    organizations ||--o{ audit_events : records
    sessions ||--o{ audit_events : relates
    operators ||--o{ audit_events : acts
    hosts ||--o{ audit_events : acts

    organizations {
        uuid id PK
        text name
    }
    units {
        uuid id PK
        uuid organization_id FK
        text name
    }
    operators {
        uuid id PK
        text email UK
        text status
    }
    organization_memberships {
        uuid organization_id PK,FK
        uuid operator_id PK,FK
        text role
    }
    hosts {
        uuid id PK
        uuid organization_id FK
        uuid unit_id FK
        text status
        text trust_state
    }
    host_credentials {
        uuid organization_id PK,FK
        uuid host_id PK,FK
        int key_version PK
        text algorithm
        text public_key
        text identity_fingerprint UK
    }
    sessions {
        uuid id PK
        uuid organization_id FK
        uuid host_id FK
        uuid requested_by FK
        text status
    }
    session_participants {
        uuid organization_id PK,FK
        uuid session_id PK,FK
        uuid operator_id PK,FK
        text role
    }
    audit_events {
        uuid id PK
        uuid organization_id FK
        uuid session_id FK
        uuid correlation_id
        text event_type
        jsonb metadata
    }
\`\`\`

## Identidade do Host

\`host_credentials\` armazena somente material público. A proposta da versão 1 é Ed25519, chave pública bruta codificada em base64url sem padding (\`key_encoding = base64url-raw\`) e fingerprint operacional globalmente único. A chave privada permanece no Host.

O registro não equivale a confiança. O ciclo esperado é:

1. criar o Host em estado \`unverified\`;
2. executar enrollment com prova de posse;
3. registrar \`proof_of_possession_at\`;
4. mudar \`trust_state\` para \`trusted\` somente após validação central;
5. rotacionar inserindo nova versão e revogando a anterior;
6. marcar o Host como \`revoked\` quando a identidade não puder mais ser aceita.

Geração de chaves, prova de posse e armazenamento seguro ainda não são implementados nesta missão.

## Exclusão e retenção

As foreign keys usam comportamento restritivo por padrão. Exclusão física de organizações, Hosts, sessões e eventos não faz parte do fluxo operacional. Estados (\`suspended\`, \`revoked\`, \`ended\`) devem ser usados para ciclo de vida.

\`audit_events\` possui trigger de append-only para UPDATE e DELETE. Isso não é imutabilidade absoluta: superusuários e administradores do PostgreSQL continuam podendo alterar a infraestrutura. Retenção, arquivamento e backup devem ser operados por procedimento controlado fora do papel da aplicação.
