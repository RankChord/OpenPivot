CREATE TABLE IF NOT EXISTS flows (
    id UUID PRIMARY KEY,
    space_id UUID NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_flows_space
ON flows (space_id);

CREATE INDEX IF NOT EXISTS idx_flows_created_by
ON flows (created_by);