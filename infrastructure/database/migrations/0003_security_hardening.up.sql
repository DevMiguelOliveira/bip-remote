-- Migration 0003: Hardening de segurança, propriedade e privilégios mínimos
-- Estratégia de ownership: bip_migrator é owner dos objetos de aplicação (Alternativa A).
-- bip_app: role de runtime, sem DDL, sem BYPASSRLS, sem propriedade de tabelas.
-- Não concede GRANT ALL a PUBLIC nem privilégios genéricos.

-- 1. Hardening de schemas: revogar do PUBLIC, conceder USAGE apenas onde necessário
REVOKE ALL ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA public TO bip_migrator;
GRANT USAGE ON SCHEMA public TO bip_app;

GRANT USAGE ON SCHEMA app TO bip_migrator;
GRANT USAGE ON SCHEMA app TO bip_app;

-- 2. Privilegios de schema para bip_migrator (capacidade real de executar migrations DDL)
GRANT CREATE ON SCHEMA public TO bip_migrator;
GRANT CREATE ON SCHEMA app TO bip_migrator;

-- 3. Transferencia de ownership para bip_migrator (objetos criados por 0001 e 0002)
-- Tabelas
ALTER TABLE public.organizations OWNER TO bip_migrator;
ALTER TABLE public.units OWNER TO bip_migrator;
ALTER TABLE public.operators OWNER TO bip_migrator;
ALTER TABLE public.organization_memberships OWNER TO bip_migrator;
ALTER TABLE public.hosts OWNER TO bip_migrator;
ALTER TABLE public.host_credentials OWNER TO bip_migrator;
ALTER TABLE public.sessions OWNER TO bip_migrator;
ALTER TABLE public.session_participants OWNER TO bip_migrator;
ALTER TABLE public.audit_events OWNER TO bip_migrator;

-- Sequences (colunas id UUID com default gen_random_uuid nao criam sequences próprias;
-- mantemos por completude caso existam)
-- Nenhuma sequence explícita criada por 0001/0002 (UUID usa gen_random_uuid).

-- Funcoes do schema app
ALTER FUNCTION app.current_organization_id() OWNER TO bip_migrator;
ALTER FUNCTION app.prevent_audit_events_mutation() OWNER TO bip_migrator;

-- 4. Privilegios explícitos para bip_app (runtime mínimo, menos privilegio)
-- DML em tabelas de negocio
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE public.organizations TO bip_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE public.units TO bip_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE public.operators TO bip_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE public.organization_memberships TO bip_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE public.hosts TO bip_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE public.host_credentials TO bip_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE public.sessions TO bip_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE public.session_participants TO bip_app;

-- audit_events e append-only: apenas SELECT e INSERT, sem UPDATE/DELETE
GRANT SELECT, INSERT ON TABLE public.audit_events TO bip_app;

-- 5. Harding da funcao de contexto RLS
ALTER FUNCTION app.current_organization_id() SET search_path = pg_catalog;
REVOKE ALL ON FUNCTION app.current_organization_id() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION app.current_organization_id() TO bip_app;
GRANT EXECUTE ON FUNCTION app.current_organization_id() TO bip_migrator;

-- 6. Preservar RLS e FORCE ROW LEVEL SECURITY (ja criados pela 0002; nao alterar semanticamente)
-- bip_app nao e proprietario das tabelas (transferido acima para bip_migrator)
-- bip_app nao possui BYPASSRLS (nao concedido)
-- bip_app nao possui CREATE em public nem em app (apenas USAGE)
