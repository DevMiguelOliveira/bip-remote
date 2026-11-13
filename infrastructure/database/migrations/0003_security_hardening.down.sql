-- Rollback da Migration 0003: Hardening de seguranca e privilegios minimos
-- Reverte EXATAMENTE o que o UP introduziu.
--
-- LIMITACOES DE REVERSIBILIDADE:
--   - O owner anterior dos objetos nao e gravado pelo UP. Como 0001 e 0002 nao
--     definem owner explicito, assumimos que o owner era o usuario que executou
--     aquelas migrações (tipicamente 'postgres'). O DOWN restaura para 'postgres'.
--   - ACLs anteriores (antes da 0003) nao sao conhecidas. O DOWN nao as reconstrói;
--     apenas remove as concessoes introduzidas pelo UP.
--   - Nao remove roles (bip_migrator, bip_app permanecem no cluster).
--   - Nao concede ALL a PUBLIC.

-- 1. Reverter funcao de contexto RLS
REVOKE EXECUTE ON FUNCTION app.current_organization_id() FROM bip_app;
REVOKE EXECUTE ON FUNCTION app.current_organization_id() FROM bip_migrator;
ALTER FUNCTION app.current_organization_id() RESET search_path;

-- 2. Revogar privilegios de bip_app nas tabelas (DML)
REVOKE SELECT, INSERT, UPDATE, DELETE ON TABLE public.organizations FROM bip_app;
REVOKE SELECT, INSERT, UPDATE, DELETE ON TABLE public.units FROM bip_app;
REVOKE SELECT, INSERT, UPDATE, DELETE ON TABLE public.operators FROM bip_app;
REVOKE SELECT, INSERT, UPDATE, DELETE ON TABLE public.organization_memberships FROM bip_app;
REVOKE SELECT, INSERT, UPDATE, DELETE ON TABLE public.hosts FROM bip_app;
REVOKE SELECT, INSERT, UPDATE, DELETE ON TABLE public.host_credentials FROM bip_app;
REVOKE SELECT, INSERT, UPDATE, DELETE ON TABLE public.sessions FROM bip_app;
REVOKE SELECT, INSERT, UPDATE, DELETE ON TABLE public.session_participants FROM bip_app;
REVOKE SELECT, INSERT ON TABLE public.audit_events FROM bip_app;

-- 3. Restaurar ownership das tabelas para o estado anterior (assumido: postgres)
ALTER TABLE public.organizations OWNER TO postgres;
ALTER TABLE public.units OWNER TO postgres;
ALTER TABLE public.operators OWNER TO postgres;
ALTER TABLE public.organization_memberships OWNER TO postgres;
ALTER TABLE public.hosts OWNER TO postgres;
ALTER TABLE public.host_credentials OWNER TO postgres;
ALTER TABLE public.sessions OWNER TO postgres;
ALTER TABLE public.session_participants OWNER TO postgres;
ALTER TABLE public.audit_events OWNER TO postgres;

-- 4. Restaurar ownership das funcoes para o estado anterior
ALTER FUNCTION app.current_organization_id() OWNER TO postgres;
ALTER FUNCTION app.prevent_audit_events_mutation() OWNER TO postgres;

-- 5. Revogar privilegios de schema de bip_migrator (CREATE)
REVOKE CREATE ON SCHEMA public FROM bip_migrator;
REVOKE CREATE ON SCHEMA app FROM bip_migrator;

-- 6. Revogar USAGE de schemas de bip_app e bip_migrator
REVOKE USAGE ON SCHEMA app FROM bip_app;
REVOKE USAGE ON SCHEMA app FROM bip_migrator;
REVOKE USAGE ON SCHEMA public FROM bip_app;
REVOKE USAGE ON SCHEMA public FROM bip_migrator;

-- Observacao: Nao concedemos USAGE ou CREATE a PUBLIC. O estado final de schemas
-- apos o DOWN e conservador (sem concessoes amplas a PUBLIC).
