CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE units (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id),
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, name)
);

CREATE TABLE operators (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL CHECK (length(trim(email)) > 3),
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('invited', 'active', 'suspended', 'revoked')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE organization_memberships (
    organization_id UUID NOT NULL REFERENCES organizations(id),
    operator_id UUID NOT NULL REFERENCES operators(id),
    role TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'technician', 'auditor')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (organization_id, operator_id)
);

CREATE TABLE hosts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id),
    unit_id UUID REFERENCES units(id),
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    hostname TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'online', 'offline', 'revoked')),
    last_seen_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE host_credentials (
    host_id UUID PRIMARY KEY REFERENCES hosts(id),
    public_key TEXT NOT NULL,
    identity_fingerprint TEXT NOT NULL UNIQUE,
    issued_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ,
    last_rotated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id),
    host_id UUID NOT NULL REFERENCES hosts(id),
    requested_by UUID NOT NULL REFERENCES operators(id),
    status TEXT NOT NULL DEFAULT 'requested' CHECK (status IN ('requested', 'approved', 'active', 'ended', 'revoked')),
    reason TEXT CHECK (reason IS NULL OR length(reason) <= 500),
    started_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE session_participants (
    session_id UUID NOT NULL REFERENCES sessions(id),
    operator_id UUID NOT NULL REFERENCES operators(id),
    role TEXT NOT NULL CHECK (role IN ('controller', 'observer', 'approver')),
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    left_at TIMESTAMPTZ,
    PRIMARY KEY (session_id, operator_id)
);

CREATE TABLE audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id),
    session_id UUID REFERENCES sessions(id),
    actor_operator_id UUID REFERENCES operators(id),
    actor_host_id UUID REFERENCES hosts(id),
    event_type TEXT NOT NULL,
    result TEXT NOT NULL CHECK (result IN ('accepted', 'denied', 'failed')),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX units_organization_id_idx ON units (organization_id);
CREATE UNIQUE INDEX operators_email_lower_idx ON operators (lower(email));
CREATE INDEX hosts_organization_id_status_idx ON hosts (organization_id, status);
CREATE INDEX hosts_unit_id_idx ON hosts (unit_id);
CREATE INDEX sessions_host_id_status_idx ON sessions (host_id, status);
CREATE INDEX sessions_requested_by_idx ON sessions (requested_by);
CREATE INDEX audit_events_organization_occurred_idx ON audit_events (organization_id, occurred_at DESC);
CREATE INDEX audit_events_session_occurred_idx ON audit_events (session_id, occurred_at DESC);
