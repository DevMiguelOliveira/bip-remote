DROP POLICY IF EXISTS audit_events_tenant_isolation ON audit_events;
DROP POLICY IF EXISTS session_participants_tenant_isolation ON session_participants;
DROP POLICY IF EXISTS sessions_tenant_isolation ON sessions;
DROP POLICY IF EXISTS host_credentials_tenant_isolation ON host_credentials;
DROP POLICY IF EXISTS hosts_tenant_isolation ON hosts;
DROP POLICY IF EXISTS organization_memberships_tenant_isolation ON organization_memberships;
DROP POLICY IF EXISTS operators_membership_select ON operators;
DROP POLICY IF EXISTS operators_membership_update ON operators;
DROP POLICY IF EXISTS operators_membership_delete ON operators;
DROP POLICY IF EXISTS operators_provision_with_context ON operators;
DROP POLICY IF EXISTS units_tenant_isolation ON units;
DROP POLICY IF EXISTS organizations_tenant_isolation ON organizations;

ALTER TABLE audit_events NO FORCE ROW LEVEL SECURITY;
ALTER TABLE audit_events DISABLE ROW LEVEL SECURITY;
ALTER TABLE session_participants NO FORCE ROW LEVEL SECURITY;
ALTER TABLE session_participants DISABLE ROW LEVEL SECURITY;
ALTER TABLE sessions NO FORCE ROW LEVEL SECURITY;
ALTER TABLE sessions DISABLE ROW LEVEL SECURITY;
ALTER TABLE host_credentials NO FORCE ROW LEVEL SECURITY;
ALTER TABLE host_credentials DISABLE ROW LEVEL SECURITY;
ALTER TABLE hosts NO FORCE ROW LEVEL SECURITY;
ALTER TABLE hosts DISABLE ROW LEVEL SECURITY;
ALTER TABLE organization_memberships NO FORCE ROW LEVEL SECURITY;
ALTER TABLE organization_memberships DISABLE ROW LEVEL SECURITY;
ALTER TABLE operators NO FORCE ROW LEVEL SECURITY;
ALTER TABLE operators DISABLE ROW LEVEL SECURITY;
ALTER TABLE units NO FORCE ROW LEVEL SECURITY;
ALTER TABLE units DISABLE ROW LEVEL SECURITY;
ALTER TABLE organizations NO FORCE ROW LEVEL SECURITY;
ALTER TABLE organizations DISABLE ROW LEVEL SECURITY;

DROP TRIGGER IF EXISTS audit_events_append_only_trg ON audit_events;
DROP FUNCTION IF EXISTS app.prevent_audit_events_mutation();

ALTER TABLE audit_events
    DROP CONSTRAINT audit_events_organization_session_fkey,
    DROP CONSTRAINT audit_events_organization_actor_operator_fkey,
    DROP CONSTRAINT audit_events_organization_actor_host_fkey,
    ADD CONSTRAINT audit_events_session_id_fkey
        FOREIGN KEY (session_id) REFERENCES sessions (id),
    ADD CONSTRAINT audit_events_actor_operator_id_fkey
        FOREIGN KEY (actor_operator_id) REFERENCES operators (id),
    ADD CONSTRAINT audit_events_actor_host_id_fkey
        FOREIGN KEY (actor_host_id) REFERENCES hosts (id),
    DROP CONSTRAINT audit_events_schema_version_chk,
    DROP CONSTRAINT audit_events_event_type_nonempty_chk,
    DROP CONSTRAINT audit_events_metadata_object_chk,
    DROP CONSTRAINT audit_events_actor_exclusive_chk,
    DROP CONSTRAINT audit_events_resource_pair_chk;

ALTER TABLE audit_events
    DROP COLUMN resource_type,
    DROP COLUMN resource_id,
    DROP COLUMN correlation_id,
    DROP COLUMN schema_version;

ALTER TABLE session_participants
    DROP CONSTRAINT session_participants_organization_session_fkey,
    DROP CONSTRAINT session_participants_organization_operator_fkey,
    DROP CONSTRAINT session_participants_pkey,
    ADD CONSTRAINT session_participants_pkey PRIMARY KEY (session_id, operator_id),
    DROP COLUMN organization_id,
    ADD CONSTRAINT session_participants_session_id_fkey
        FOREIGN KEY (session_id) REFERENCES sessions (id),
    ADD CONSTRAINT session_participants_operator_id_fkey
        FOREIGN KEY (operator_id) REFERENCES operators (id);

ALTER TABLE sessions
    DROP CONSTRAINT sessions_organization_host_fkey,
    DROP CONSTRAINT sessions_organization_requester_membership_fkey,
    ADD CONSTRAINT sessions_host_id_fkey
        FOREIGN KEY (host_id) REFERENCES hosts (id),
    ADD CONSTRAINT sessions_requested_by_fkey
        FOREIGN KEY (requested_by) REFERENCES operators (id),
    DROP CONSTRAINT sessions_organization_id_id_key,
    DROP CONSTRAINT sessions_time_order_chk;

DROP INDEX IF EXISTS audit_events_organization_correlation_idx;
DROP INDEX IF EXISTS host_credentials_one_active_idx;
DROP INDEX IF EXISTS session_participants_organization_session_idx;
DROP INDEX IF EXISTS sessions_organization_status_idx;
DROP INDEX IF EXISTS organization_memberships_operator_id_idx;
DROP INDEX IF EXISTS hosts_organization_hostname_lower_idx;
DROP INDEX IF EXISTS hosts_organization_display_name_lower_idx;
DROP INDEX IF EXISTS units_organization_name_lower_idx;

ALTER TABLE host_credentials
    DROP CONSTRAINT host_credentials_organization_host_fkey,
    DROP CONSTRAINT host_credentials_pkey,
    ADD CONSTRAINT host_credentials_pkey PRIMARY KEY (host_id),
    DROP CONSTRAINT host_credentials_key_version_chk,
    DROP CONSTRAINT host_credentials_algorithm_chk,
    DROP CONSTRAINT host_credentials_key_encoding_chk,
    DROP CONSTRAINT host_credentials_public_key_nonempty_chk,
    DROP CONSTRAINT host_credentials_fingerprint_nonempty_chk,
    DROP COLUMN organization_id,
    DROP COLUMN key_version,
    DROP COLUMN algorithm,
    DROP COLUMN key_encoding,
    DROP COLUMN proof_of_possession_at,
    DROP COLUMN valid_from,
    ADD CONSTRAINT host_credentials_host_id_fkey
        FOREIGN KEY (host_id) REFERENCES hosts (id);

ALTER TABLE hosts
    DROP CONSTRAINT hosts_organization_unit_fkey,
    ADD CONSTRAINT hosts_unit_id_fkey
        FOREIGN KEY (unit_id) REFERENCES units (id),
    DROP CONSTRAINT hosts_organization_id_id_key,
    DROP CONSTRAINT hosts_trust_state_chk,
    DROP COLUMN trust_state;

ALTER TABLE units
    DROP CONSTRAINT units_organization_id_id_key;

DROP FUNCTION IF EXISTS app.current_organization_id();
DROP SCHEMA IF EXISTS app;
