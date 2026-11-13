CREATE SCHEMA IF NOT EXISTS app;

CREATE OR REPLACE FUNCTION app.current_organization_id()
RETURNS UUID
LANGUAGE SQL
STABLE
AS $$
    SELECT NULLIF(current_setting('app.current_organization_id', true), '')::UUID
$$;

ALTER TABLE units
    ADD CONSTRAINT units_organization_id_id_key UNIQUE (organization_id, id);

ALTER TABLE hosts
    ADD CONSTRAINT hosts_organization_id_id_key UNIQUE (organization_id, id),
    ADD COLUMN trust_state TEXT NOT NULL DEFAULT 'unverified',
    ADD CONSTRAINT hosts_trust_state_chk
        CHECK (trust_state IN ('unverified', 'trusted', 'suspended', 'revoked'));

ALTER TABLE sessions
    ADD CONSTRAINT sessions_organization_id_id_key UNIQUE (organization_id, id),
    ADD CONSTRAINT sessions_time_order_chk
        CHECK (ended_at IS NULL OR started_at IS NULL OR ended_at >= started_at);

CREATE UNIQUE INDEX units_organization_name_lower_idx
    ON units (organization_id, lower(name));

CREATE UNIQUE INDEX hosts_organization_display_name_lower_idx
    ON hosts (organization_id, lower(display_name));

CREATE UNIQUE INDEX hosts_organization_hostname_lower_idx
    ON hosts (organization_id, lower(hostname))
    WHERE hostname IS NOT NULL;

CREATE INDEX organization_memberships_operator_id_idx
    ON organization_memberships (operator_id);

CREATE INDEX sessions_organization_status_idx
    ON sessions (organization_id, status);

ALTER TABLE hosts
    DROP CONSTRAINT hosts_unit_id_fkey,
    ADD CONSTRAINT hosts_organization_unit_fkey
        FOREIGN KEY (organization_id, unit_id)
        REFERENCES units (organization_id, id);

ALTER TABLE sessions
    DROP CONSTRAINT sessions_host_id_fkey,
    DROP CONSTRAINT sessions_requested_by_fkey,
    ADD CONSTRAINT sessions_organization_host_fkey
        FOREIGN KEY (organization_id, host_id)
        REFERENCES hosts (organization_id, id),
    ADD CONSTRAINT sessions_organization_requester_membership_fkey
        FOREIGN KEY (organization_id, requested_by)
        REFERENCES organization_memberships (organization_id, operator_id);

ALTER TABLE host_credentials
    ADD COLUMN organization_id UUID,
    ADD COLUMN key_version INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN algorithm TEXT NOT NULL DEFAULT 'ed25519',
    ADD COLUMN key_encoding TEXT NOT NULL DEFAULT 'base64url-raw',
    ADD COLUMN proof_of_possession_at TIMESTAMPTZ,
    ADD COLUMN valid_from TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD CONSTRAINT host_credentials_key_version_chk CHECK (key_version > 0),
    ADD CONSTRAINT host_credentials_algorithm_chk CHECK (algorithm IN ('ed25519')),
    ADD CONSTRAINT host_credentials_key_encoding_chk CHECK (key_encoding IN ('base64url-raw')),
    ADD CONSTRAINT host_credentials_public_key_nonempty_chk
        CHECK (length(trim(public_key)) > 0),
    ADD CONSTRAINT host_credentials_fingerprint_nonempty_chk
        CHECK (length(trim(identity_fingerprint)) > 0);

UPDATE host_credentials credentials
SET organization_id = hosts.organization_id
FROM hosts
WHERE hosts.id = credentials.host_id;

ALTER TABLE host_credentials
    ALTER COLUMN organization_id SET NOT NULL,
    DROP CONSTRAINT host_credentials_host_id_fkey,
    DROP CONSTRAINT host_credentials_pkey,
    ADD CONSTRAINT host_credentials_pkey
        PRIMARY KEY (organization_id, host_id, key_version),
    ADD CONSTRAINT host_credentials_organization_host_fkey
        FOREIGN KEY (organization_id, host_id)
        REFERENCES hosts (organization_id, id);

CREATE UNIQUE INDEX host_credentials_one_active_idx
    ON host_credentials (organization_id, host_id)
    WHERE revoked_at IS NULL;

ALTER TABLE session_participants
    ADD COLUMN organization_id UUID;

UPDATE session_participants participants
SET organization_id = sessions.organization_id
FROM sessions
WHERE sessions.id = participants.session_id;

ALTER TABLE session_participants
    ALTER COLUMN organization_id SET NOT NULL,
    DROP CONSTRAINT session_participants_session_id_fkey,
    DROP CONSTRAINT session_participants_operator_id_fkey,
    DROP CONSTRAINT session_participants_pkey,
    ADD CONSTRAINT session_participants_pkey
        PRIMARY KEY (organization_id, session_id, operator_id),
    ADD CONSTRAINT session_participants_organization_session_fkey
        FOREIGN KEY (organization_id, session_id)
        REFERENCES sessions (organization_id, id),
    ADD CONSTRAINT session_participants_organization_operator_fkey
        FOREIGN KEY (organization_id, operator_id)
        REFERENCES organization_memberships (organization_id, operator_id);

CREATE INDEX session_participants_organization_session_idx
    ON session_participants (organization_id, session_id);

ALTER TABLE audit_events
    ADD COLUMN resource_type TEXT,
    ADD COLUMN resource_id UUID,
    ADD COLUMN correlation_id UUID NOT NULL DEFAULT gen_random_uuid(),
    ADD COLUMN schema_version INTEGER,
    ADD CONSTRAINT audit_events_schema_version_placeholder_chk CHECK (schema_version IS NULL OR schema_version > 0),
    ADD CONSTRAINT audit_events_event_type_nonempty_chk
        CHECK (length(trim(event_type)) > 0),
    ADD CONSTRAINT audit_events_metadata_object_chk
        CHECK (jsonb_typeof(metadata) = 'object'),
    ADD CONSTRAINT audit_events_actor_exclusive_chk
        CHECK (NOT (actor_operator_id IS NOT NULL AND actor_host_id IS NOT NULL)),
    ADD CONSTRAINT audit_events_resource_pair_chk
        CHECK (
            (resource_type IS NULL AND resource_id IS NULL)
            OR (
                resource_type IS NOT NULL
                AND length(trim(resource_type)) > 0
                AND resource_id IS NOT NULL
            )
        );

UPDATE audit_events
SET schema_version = 1;

ALTER TABLE audit_events
    ALTER COLUMN schema_version SET DEFAULT 2,
    ALTER COLUMN schema_version SET NOT NULL,
    DROP CONSTRAINT audit_events_schema_version_placeholder_chk,
    ADD CONSTRAINT audit_events_schema_version_chk CHECK (schema_version > 0);

ALTER TABLE audit_events
    DROP CONSTRAINT audit_events_session_id_fkey,
    DROP CONSTRAINT audit_events_actor_operator_id_fkey,
    DROP CONSTRAINT audit_events_actor_host_id_fkey,
    ADD CONSTRAINT audit_events_organization_session_fkey
        FOREIGN KEY (organization_id, session_id)
        REFERENCES sessions (organization_id, id),
    ADD CONSTRAINT audit_events_organization_actor_operator_fkey
        FOREIGN KEY (organization_id, actor_operator_id)
        REFERENCES organization_memberships (organization_id, operator_id),
    ADD CONSTRAINT audit_events_organization_actor_host_fkey
        FOREIGN KEY (organization_id, actor_host_id)
        REFERENCES hosts (organization_id, id);

CREATE INDEX audit_events_organization_correlation_idx
    ON audit_events (organization_id, correlation_id);

CREATE OR REPLACE FUNCTION app.prevent_audit_events_mutation()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'audit_events is append-only';
END;
$$;

DROP TRIGGER IF EXISTS audit_events_append_only_trg ON audit_events;

CREATE TRIGGER audit_events_append_only_trg
BEFORE UPDATE OR DELETE ON audit_events
FOR EACH ROW
EXECUTE FUNCTION app.prevent_audit_events_mutation();

ALTER TABLE organizations ENABLE ROW LEVEL SECURITY;
ALTER TABLE organizations FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS organizations_tenant_isolation ON organizations;
CREATE POLICY organizations_tenant_isolation ON organizations
    FOR ALL
    USING (id = app.current_organization_id())
    WITH CHECK (id = app.current_organization_id());

ALTER TABLE units ENABLE ROW LEVEL SECURITY;
ALTER TABLE units FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS units_tenant_isolation ON units;
CREATE POLICY units_tenant_isolation ON units
    FOR ALL
    USING (organization_id = app.current_organization_id())
    WITH CHECK (organization_id = app.current_organization_id());

ALTER TABLE operators ENABLE ROW LEVEL SECURITY;
ALTER TABLE operators FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS operators_membership_select ON operators;
DROP POLICY IF EXISTS operators_membership_update ON operators;
DROP POLICY IF EXISTS operators_membership_delete ON operators;
DROP POLICY IF EXISTS operators_provision_with_context ON operators;
CREATE POLICY operators_membership_select ON operators
    FOR SELECT
    USING (
        EXISTS (
            SELECT 1
            FROM organization_memberships memberships
            WHERE memberships.organization_id = app.current_organization_id()
              AND memberships.operator_id = operators.id
        )
    );
CREATE POLICY operators_membership_update ON operators
    FOR UPDATE
    USING (
        EXISTS (
            SELECT 1
            FROM organization_memberships memberships
            WHERE memberships.organization_id = app.current_organization_id()
              AND memberships.operator_id = operators.id
        )
    )
    WITH CHECK (
        EXISTS (
            SELECT 1
            FROM organization_memberships memberships
            WHERE memberships.organization_id = app.current_organization_id()
              AND memberships.operator_id = operators.id
        )
    );
CREATE POLICY operators_membership_delete ON operators
    FOR DELETE
    USING (
        EXISTS (
            SELECT 1
            FROM organization_memberships memberships
            WHERE memberships.organization_id = app.current_organization_id()
              AND memberships.operator_id = operators.id
        )
    );
CREATE POLICY operators_provision_with_context ON operators
    FOR INSERT
    WITH CHECK (app.current_organization_id() IS NOT NULL);

ALTER TABLE organization_memberships ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization_memberships FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS organization_memberships_tenant_isolation ON organization_memberships;
CREATE POLICY organization_memberships_tenant_isolation ON organization_memberships
    FOR ALL
    USING (organization_id = app.current_organization_id())
    WITH CHECK (organization_id = app.current_organization_id());

ALTER TABLE hosts ENABLE ROW LEVEL SECURITY;
ALTER TABLE hosts FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS hosts_tenant_isolation ON hosts;
CREATE POLICY hosts_tenant_isolation ON hosts
    FOR ALL
    USING (organization_id = app.current_organization_id())
    WITH CHECK (organization_id = app.current_organization_id());

ALTER TABLE host_credentials ENABLE ROW LEVEL SECURITY;
ALTER TABLE host_credentials FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS host_credentials_tenant_isolation ON host_credentials;
CREATE POLICY host_credentials_tenant_isolation ON host_credentials
    FOR ALL
    USING (organization_id = app.current_organization_id())
    WITH CHECK (organization_id = app.current_organization_id());

ALTER TABLE sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE sessions FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS sessions_tenant_isolation ON sessions;
CREATE POLICY sessions_tenant_isolation ON sessions
    FOR ALL
    USING (organization_id = app.current_organization_id())
    WITH CHECK (organization_id = app.current_organization_id());

ALTER TABLE session_participants ENABLE ROW LEVEL SECURITY;
ALTER TABLE session_participants FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS session_participants_tenant_isolation ON session_participants;
CREATE POLICY session_participants_tenant_isolation ON session_participants
    FOR ALL
    USING (organization_id = app.current_organization_id())
    WITH CHECK (organization_id = app.current_organization_id());

ALTER TABLE audit_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_events FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS audit_events_tenant_isolation ON audit_events;
CREATE POLICY audit_events_tenant_isolation ON audit_events
    FOR ALL
    USING (organization_id = app.current_organization_id())
    WITH CHECK (organization_id = app.current_organization_id());

