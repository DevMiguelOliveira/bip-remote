\set ON_ERROR_STOP on

BEGIN;
SET LOCAL ROLE bipremote_rls_test;

-- Contexto da organizacao A
SELECT set_config('app.current_organization_id', '00000000-0000-0000-0000-0000000000a1', true);

INSERT INTO organizations (id, name)
VALUES ('00000000-0000-0000-0000-0000000000a1', 'Organization A');

INSERT INTO operators (id, email, display_name)
VALUES ('00000000-0000-0000-0000-0000000000b1', 'operator-a@example.test', 'Operator A');

INSERT INTO organization_memberships (organization_id, operator_id, role)
VALUES ('00000000-0000-0000-0000-0000000000a1', '00000000-0000-0000-0000-0000000000b1', 'owner');

INSERT INTO units (id, organization_id, name)
VALUES ('00000000-0000-0000-0000-0000000000c1', '00000000-0000-0000-0000-0000000000a1', 'Unit A');

INSERT INTO hosts (id, organization_id, unit_id, display_name, hostname, status, trust_state)
VALUES (
    '00000000-0000-0000-0000-0000000000d1',
    '00000000-0000-0000-0000-0000000000a1',
    '00000000-0000-0000-0000-0000000000c1',
    'Host A',
    'host-a',
    'offline',
    'trusted'
);

INSERT INTO host_credentials (
    organization_id,
    host_id,
    key_version,
    public_key,
    identity_fingerprint,
    proof_of_possession_at
)
VALUES (
    '00000000-0000-0000-0000-0000000000a1',
    '00000000-0000-0000-0000-0000000000d1',
    1,
    'public-key-a',
    'sha256:fingerprint-a',
    now()
);

INSERT INTO sessions (id, organization_id, host_id, requested_by, reason)
VALUES (
    '00000000-0000-0000-0000-0000000000e1',
    '00000000-0000-0000-0000-0000000000a1',
    '00000000-0000-0000-0000-0000000000d1',
    '00000000-0000-0000-0000-0000000000b1',
    'validation'
);

INSERT INTO session_participants (organization_id, session_id, operator_id, role)
VALUES (
    '00000000-0000-0000-0000-0000000000a1',
    '00000000-0000-0000-0000-0000000000e1',
    '00000000-0000-0000-0000-0000000000b1',
    'controller'
);

INSERT INTO audit_events (
    organization_id,
    session_id,
    actor_operator_id,
    event_type,
    result,
    resource_type,
    resource_id
)
VALUES (
    '00000000-0000-0000-0000-0000000000a1',
    '00000000-0000-0000-0000-0000000000e1',
    '00000000-0000-0000-0000-0000000000b1',
    'session_requested',
    'accepted',
    'session',
    '00000000-0000-0000-0000-0000000000e1'
);

-- Contexto da organizacao B
SELECT set_config('app.current_organization_id', '00000000-0000-0000-0000-0000000000a2', true);

INSERT INTO organizations (id, name)
VALUES ('00000000-0000-0000-0000-0000000000a2', 'Organization B');

INSERT INTO operators (id, email, display_name)
VALUES ('00000000-0000-0000-0000-0000000000b2', 'operator-b@example.test', 'Operator B');

INSERT INTO organization_memberships (organization_id, operator_id, role)
VALUES ('00000000-0000-0000-0000-0000000000a2', '00000000-0000-0000-0000-0000000000b2', 'technician');

INSERT INTO units (id, organization_id, name)
VALUES ('00000000-0000-0000-0000-0000000000c2', '00000000-0000-0000-0000-0000000000a2', 'Unit B');

INSERT INTO hosts (id, organization_id, unit_id, display_name, hostname, status, trust_state)
VALUES (
    '00000000-0000-0000-0000-0000000000d2',
    '00000000-0000-0000-0000-0000000000a2',
    '00000000-0000-0000-0000-0000000000c2',
    'Host B',
    'host-b',
    'offline',
    'trusted'
);

-- Teste 1: Tenant B ve apenas seu proprio Host
DO $$
DECLARE
    visible_hosts INTEGER;
BEGIN
    SELECT count(*) INTO visible_hosts FROM hosts;
    IF visible_hosts <> 1 THEN
        RAISE EXCEPTION 'tenant B should see exactly one Host, got %', visible_hosts;
    END IF;
END;
$$;

-- Teste 2: FK composta cruzada e rejeitada (Host B com Unit A)
DO $$
DECLARE
    rejected BOOLEAN := false;
BEGIN
    BEGIN
        INSERT INTO hosts (id, organization_id, unit_id, display_name, status)
        VALUES (
            '00000000-0000-0000-0000-0000000000d3',
            '00000000-0000-0000-0000-0000000000a2',
            '00000000-0000-0000-0000-0000000000c1',
            'Cross Tenant Host',
            'offline'
        );
    EXCEPTION
        WHEN foreign_key_violation OR insufficient_privilege THEN
            rejected := true;
    END;
    IF NOT rejected THEN
        RAISE EXCEPTION 'cross-tenant Host to Unit association was accepted';
    END IF;
END;
$$;

-- Teste 3: Sessao cruzada rejeitada (sessao B com Host A)
DO $$
DECLARE
    rejected BOOLEAN := false;
BEGIN
    BEGIN
        INSERT INTO sessions (id, organization_id, host_id, requested_by, reason)
        VALUES (
            '00000000-0000-0000-0000-0000000000e2',
            '00000000-0000-0000-0000-0000000000a2',
            '00000000-0000-0000-0000-0000000000d1',
            '00000000-0000-0000-0000-0000000000b2',
            'cross-tenant'
        );
    EXCEPTION
        WHEN foreign_key_violation OR insufficient_privilege THEN
            rejected := true;
    END;
    IF NOT rejected THEN
        RAISE EXCEPTION 'cross-tenant session association was accepted';
    END IF;
END;
$$;

-- Teste 4: Contexto vazio nao expÃµe linhas
SELECT set_config('app.current_organization_id', '', true);

DO $$
DECLARE
    visible_hosts INTEGER;
BEGIN
    SELECT count(*) INTO visible_hosts FROM hosts;
    IF visible_hosts <> 0 THEN
        RAISE EXCEPTION 'missing tenant context exposed % Hosts', visible_hosts;
    END IF;
END;
$$;

-- Teste 5a: Contexto de organizacao inexistente (UUID valido) resulta em zero linhas (fail closed)
SELECT set_config('app.current_organization_id', '00000000-0000-0000-0000-0000000000f0', true);

DO $$
DECLARE
    visible_hosts INTEGER;
BEGIN
    SELECT count(*) INTO visible_hosts FROM hosts;
    IF visible_hosts <> 0 THEN
        RAISE EXCEPTION 'non-existent tenant context exposed % Hosts', visible_hosts;
    END IF;
END;
$$;

-- Teste 5b: Contexto de organizacao malformado (UUID invalido) falha com SQLSTATE 22P02 (invalid_text_representation)
SELECT set_config('app.current_organization_id', 'not-a-uuid', true);

DO $$
DECLARE
    caught_exception BOOLEAN := false;
BEGIN
    BEGIN
        PERFORM count(*) FROM hosts;
    EXCEPTION
        WHEN invalid_text_representation THEN
            caught_exception := true;
    END;

    IF NOT caught_exception THEN
        RAISE EXCEPTION 'malformed UUID context did not raise invalid_text_representation (22P02)';
    END IF;
END;
$$;

-- Teste 6: UPDATE cruzado rejeitado (tentar alterar Host A com contexto B)
SELECT set_config('app.current_organization_id', '00000000-0000-0000-0000-0000000000a2', true);

DO $$
DECLARE
    updated_rows INTEGER;
BEGIN
    UPDATE hosts SET display_name = 'Tampered'
    WHERE id = '00000000-0000-0000-0000-0000000000d1';
    GET DIAGNOSTICS updated_rows = ROW_COUNT;
    IF updated_rows <> 0 THEN
        RAISE EXCEPTION 'cross-tenant UPDATE accepted on Host A, % rows affected', updated_rows;
    END IF;
END;
$$;

-- Teste 7: DELETE cruzado rejeitado (tentar deletar Host A com contexto B)
DO $$
DECLARE
    deleted_rows INTEGER;
BEGIN
    DELETE FROM hosts
    WHERE id = '00000000-0000-0000-0000-0000000000d1';
    GET DIAGNOSTICS deleted_rows = ROW_COUNT;
    IF deleted_rows <> 0 THEN
        RAISE EXCEPTION 'cross-tenant DELETE accepted on Host A, % rows affected', deleted_rows;
    END IF;
END;
$$;

-- Teste 8: Trigger append-only rejeita UPDATE em audit_events
SELECT set_config('app.current_organization_id', '00000000-0000-0000-0000-0000000000a1', true);

DO $$
BEGIN
    UPDATE audit_events SET result = 'tampered'
    WHERE id = (SELECT id FROM audit_events LIMIT 1);
    RAISE EXCEPTION 'append-only trigger did not reject UPDATE';
EXCEPTION
    WHEN raise_exception THEN
        IF SQLERRM != 'audit_events is append-only' THEN
            RAISE EXCEPTION 'unexpected error: %', SQLERRM;
        END IF;
END;
$$;

-- Teste 9: Trigger append-only rejeita DELETE em audit_events
DO $$
BEGIN
    DELETE FROM audit_events
    WHERE id = (SELECT id FROM audit_events LIMIT 1);
    RAISE EXCEPTION 'append-only trigger did not reject DELETE';
EXCEPTION
    WHEN raise_exception THEN
        IF SQLERRM != 'audit_events is append-only' THEN
            RAISE EXCEPTION 'unexpected error: %', SQLERRM;
        END IF;
END;
$$;

-- Teste 10: Alteracao explicita do contexto de tenant dentro da mesma transacao
-- (nao comprova reutilizacao de conexao entre transacoes; apenas confirma que
--  a consulta utiliza o contexto B apos a alteracao explicita via set_config)
SELECT set_config('app.current_organization_id', '00000000-0000-0000-0000-0000000000a2', true);

DO $$
DECLARE
    visible_hosts INTEGER;
BEGIN
    SELECT count(*) INTO visible_hosts FROM hosts;
    IF visible_hosts <> 1 THEN
        RAISE EXCEPTION 'tenant B should see exactly one Host after context restore, got %', visible_hosts;
    END IF;
END;
$$;


-- Teste 11: Validação de privilégios e limites da role bip_app
SET LOCAL ROLE bip_app;

-- 11a. Permissões positivas: SELECT e EXECUTE da função de contexto
DO $$
DECLARE
    visible_hosts INTEGER;
    current_org UUID;
BEGIN
    SELECT app.current_organization_id() INTO current_org;
    SELECT count(*) INTO visible_hosts FROM hosts;
END;
$$;

-- 11b. Permissão negativa: DDL (ALTER TABLE negado)
DO $$
DECLARE
    caught_exception BOOLEAN := false;
BEGIN
    BEGIN
        ALTER TABLE hosts ADD COLUMN test_hardening_col TEXT;
    EXCEPTION
        WHEN insufficient_privilege THEN
            caught_exception := true;
    END;

    IF NOT caught_exception THEN
        RAISE EXCEPTION 'bip_app should not have DDL privileges (ALTER TABLE)';
    END IF;
END;
$$;

-- 11c. Permissão negativa: TRUNCATE negado em tabelas do sistema
DO $$
DECLARE
    caught_exception BOOLEAN := false;
BEGIN
    BEGIN
        TRUNCATE TABLE hosts;
    EXCEPTION
        WHEN insufficient_privilege THEN
            caught_exception := true;
    END;

    IF NOT caught_exception THEN
        RAISE EXCEPTION 'bip_app should not have TRUNCATE privileges on hosts';
    END IF;
END;
$$;

-- 11d. Permissão negativa: UPDATE em audit_events negado por falta de privilégio DML
DO $$
DECLARE
    caught_exception BOOLEAN := false;
BEGIN
    BEGIN
        UPDATE audit_events SET result = 'tampered' WHERE id = (SELECT id FROM audit_events LIMIT 1);
    EXCEPTION
        WHEN insufficient_privilege THEN
            caught_exception := true;
    END;

    IF NOT caught_exception THEN
        RAISE EXCEPTION 'bip_app should not have UPDATE privileges on audit_events';
    END IF;
END;
$$;

-- 11e. Permissão negativa: DELETE em audit_events negado por falta de privilégio DML
DO $$
DECLARE
    caught_exception BOOLEAN := false;
BEGIN
    BEGIN
        DELETE FROM audit_events WHERE id = (SELECT id FROM audit_events LIMIT 1);
    EXCEPTION
        WHEN insufficient_privilege THEN
            caught_exception := true;
    END;

    IF NOT caught_exception THEN
        RAISE EXCEPTION 'bip_app should not have DELETE privileges on audit_events';
    END IF;
END;
$$;

-- 11f. Permissao positiva: INSERT autorizado em hosts para bip_app
DO \$\$
DECLARE
    inserted_rows INTEGER;
BEGIN
    INSERT INTO hosts (id, organization_id, display_name, status)
    VALUES (
        '00000000-0000-0000-0000-0000000000d9',
        '00000000-0000-0000-0000-0000000000a1',
        'Test Host BipApp',
        'pending'
    );
    GET DIAGNOSTICS inserted_rows = ROW_COUNT;
    IF inserted_rows <> 1 THEN
        RAISE EXCEPTION 'bip_app should be able to INSERT into hosts, % rows inserted', inserted_rows;
    END IF;
END;
\$\$;

-- 11g. Verificacao: bip_app nao e proprietario das tabelas protegidas
DO \$\$
DECLARE
    table_owner TEXT;
BEGIN
    SELECT tableowner INTO table_owner
    FROM pg_catalog.pg_tables
    WHERE schemaname = 'public' AND tablename = 'hosts';
    IF table_owner = 'bip_app' THEN
        RAISE EXCEPTION 'bip_app should not own table hosts';
    END IF;
END;
\$\$;

-- 11h. Verificacao: bip_app nao possui BYPASSRLS
DO \$\$
DECLARE
    has_bypassrls BOOLEAN;
BEGIN
    SELECT rolbypassrls INTO has_bypassrls
    FROM pg_catalog.pg_roles
    WHERE rolname = 'bip_app';
    IF has_bypassrls THEN
        RAISE EXCEPTION 'bip_app should not have BYPASSRLS attribute';
    END IF;
END;
\$\$;

RESET ROLE;

ROLLBACK;

