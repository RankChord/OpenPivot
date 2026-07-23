CREATE TABLE IF NOT EXISTS conversations (
    id UUID PRIMARY KEY,
    type TEXT NOT NULL DEFAULT 'direct',
    user_low_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user_high_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT conversations_type_check
        CHECK (type IN ('direct')),

    CONSTRAINT conversations_not_self
        CHECK (user_low_id <> user_high_id),

    CONSTRAINT conversations_ordered_pair
        CHECK (user_low_id < user_high_id),

    CONSTRAINT conversations_unique_direct_pair
        UNIQUE (user_low_id, user_high_id)
);

CREATE INDEX IF NOT EXISTS idx_conversations_user_low
ON conversations (user_low_id);

CREATE INDEX IF NOT EXISTS idx_conversations_user_high
ON conversations (user_high_id);