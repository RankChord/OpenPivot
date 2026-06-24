CREATE TABLE IF NOT EXISTS space_members (
    id BIGSERIAL PRIMARY KEY,
    space_id BIGINT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL DEFAULT 'member',
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT space_members_role_check
        CHECK (role IN ('owner', 'admin', 'member')),

    CONSTRAINT space_members_unique_user
        UNIQUE (space_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_space_members_user
ON space_members (user_id);

CREATE INDEX IF NOT EXISTS idx_space_members_space
ON space_members (space_id);