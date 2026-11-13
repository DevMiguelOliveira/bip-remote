-- Initial database schema for BipRemote foundation
-- This migration creates the core tables needed for remote access functionality

-- ============================================================
-- Enum types used across multiple tables
-- ============================================================

-- Host identity status
CREATE TYPE host_identity_status AS ENUM (
    'pending',
    'active',
    'revoked',
    'expired'
);

-- Session state
CREATE TYPE session_state AS ENUM (
    'pending',
    'approved',
    'active',
    'ended',
    'failed',
    'revoked'
);

-- Connection transport method
CREATE TYPE connection_transport AS ENUM (
    'direct_p2p',
    'relayed',
    'tcp_hole_punch'
);

-- Event type enum
CREATE TYPE event_type AS ENUM (
    'connection.established',
    'connection.failed',
    'session.approved',
    'session.ended',
    'stream.frame',
    'stream.audio',
    'control.input',
    'auth.challenge',
    'auth.response'
);

-- ============================================================
-- Organizations table
-- ============================================================

CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    display_name VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- Units (organizational groups)
-- ============================================================

CREATE TABLE units (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- Operators/users
-- ============================================================

CREATE TABLE operators (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    unit_id UUID REFERENCES units(id) ON DELETE SET NULL,
    username VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE,
    display_name VARCHAR(255),
    password_hash VARCHAR(255) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_superuser BOOLEAN NOT NULL DEFAULT FALSE,
    last_login TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- Hosts (Windows machines running the BipRemote host agent)
-- ============================================================

CREATE TABLE hosts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    unit_id UUID REFERENCES units(id) ON DELETE SET NULL,
    fingerprint VARCHAR(64) NOT NULL,  -- SHA-256 hex encoded
    public_key BYTEA NOT NULL,         -- mTLS or Ed25519 public key
    status host_identity_status NOT NULL DEFAULT 'pending',
    is_privileged BOOLEAN NOT NULL DEFAULT FALSE,
    last_seen_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (fingerprint)
);

-- ============================================================
-- Sessions (remote access sessions)
-- ============================================================

CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    host_id UUID REFERENCES hosts(id) ON DELETE CASCADE,
    client_identifier VARCHAR(255) NOT NULL,
    state session_state NOT NULL DEFAULT 'pending',
    transport connection_transport NOT NULL DEFAULT 'relayed',
    authorized_at TIMESTAMPTZ,
    approved_by UUID REFERENCES operators(id) ON DELETE SET NULL,
    approved_at TIMESTAMPTZ,
    started_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    duration_seconds INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ==========================================================--
-- Session events (auditable, versioned events)
-- ============================================================

CREATE TABLE session_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID REFERENCES sessions(id) ON DELETE CASCADE,
    event_type event_type NOT NULL,
    schema_version VARCHAR(10) NOT NULL DEFAULT 'v1',
    correlation_id UUID NOT NULL,
    payload JSONB NOT NULL,
    actor_type VARCHAR(50) NOT NULL,
    actor_id UUID,
    resource_type VARCHAR(50),
    resource_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for session events queries
CREATE INDEX idx_session_events_session_id ON session_events(session_id);
CREATE INDEX idx_session_events_event_type ON session_events(event_type);
CREATE INDEX idx_session_events_created_at ON session_events(created_at);

-- ============================================================
-- Stream sessions (linked to main sessions)
-- ============================================================

CREATE TABLE stream_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID UNIQUE REFERENCES sessions(id) ON DELETE CASCADE,
    codec VARCHAR(50) NOT NULL,        -- e.g., 'h264', 'vp8', 'vp9', 'av1'
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    frame_rate NUMERIC(5,2) NOT NULL DEFAULT 30.0,
    bitrate_kbps INTEGER NOT NULL DEFAULT 2000,
    status VARCHAR(50) NOT NULL DEFAULT 'idle',  -- 'idle', 'active', 'paused', 'error'
    relay_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- API access tokens (for admin console <-> backend authentication)
-- ============================================================

CREATE TABLE api_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    operator_id UUID REFERENCES operators(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    scope VARCHAR(100) NOT NULL DEFAULT 'read',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ
);

-- ============================================================
-- Indexes and constraints
-- ============================================================

-- Organization lookups
CREATE INDEX idx_hosts_org_id ON hosts(organization_id);
CREATE INDEX idx_sessions_org_id ON sessions(organization_id);
CREATE INDEX idx_stream_sessions_org_id ON stream_sessions(organization_id);

-- Session state lookups
CREATE INDEX idx_sessions_state ON sessions(state);
CREATE INDEX idx_stream_sessions_status ON stream_sessions(status);

-- Fingerprint lookups (for quick host identification)
CREATE INDEX idx_hosts_fingerprint ON hosts(fingerprint);

-- Token lookups
CREATE INDEX idx_api_tokens_hash ON api_tokens(token_hash);
CREATE INDEX idx_api_tokens_operator ON api_tokens(operator_id);

-- ============================================================
-- Row-level security policies (will be configured in application)
-- ============================================================

-- Comment: RLS policies should be enabled and configured per-organization
-- to restrict data access based on operator roles and permissions.