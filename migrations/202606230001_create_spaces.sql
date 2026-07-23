CREATE TABLE IF NOT EXISTS spaces (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    type TEXT NOT NULL DEFAULT 'group',
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT spaces_type_check
        CHECK (type IN ('group', 'workflow'))
);

CREATE INDEX IF NOT EXISTS idx_spaces_owner
ON spaces (owner_id);