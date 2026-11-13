-- Bootstrap administrativo de roles para o BIPRemote
-- Executar este script uma única vez por administrador do cluster.
-- NAO executar dentro de migrations.

DO $$
BEGIN
    -- Criar ou validar role bip_migrator
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'bip_migrator') THEN
        EXECUTE 'CREATE ROLE bip_migrator NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOINHERIT NOBYPASSRLS';
    ELSE
        -- Validar atributos de segurança
        IF EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'bip_migrator' 
                   AND (rolcanlogin OR rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolinherit OR rolbypassrls)) THEN
            RAISE EXCEPTION 'Role bip_migrator existente possui atributos de segurança inválidos (LOGIN, SUPERUSER, BYPASSRLS, etc.)';
        END IF;
    END IF;

    -- Criar ou validar role bip_app
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'bip_app') THEN
        EXECUTE 'CREATE ROLE bip_app NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOINHERIT NOBYPASSRLS';
    ELSE
        -- Validar atributos de segurança
        IF EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'bip_app' 
                   AND (rolcanlogin OR rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolinherit OR rolbypassrls)) THEN
            RAISE EXCEPTION 'Role bip_app existente possui atributos de segurança inválidos (LOGIN, SUPERUSER, BYPASSRLS, etc.)';
        END IF;
    END IF;
END
$$;
