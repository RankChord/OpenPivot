CREATE TABLE IF NOT EXISTS space_messages (
    id BIGSERIAL PRIMARY KEY,
    space_id BIGINT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    sender_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_space_messages_space_created
ON space_messages (space_id, created_at ASC);

CREATE INDEX IF NOT EXISTS idx_space_messages_sender
ON space_messages (sender_id);