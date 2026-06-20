CREATE TABLE IF NOT EXISTS friendships (
    id BIGSERIAL PRIMARY KEY,
    user_low_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user_high_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT friendships_not_self
        CHECK (user_low_id <> user_high_id),

    CONSTRAINT friendships_ordered_pair
        CHECK (user_low_id < user_high_id),

    CONSTRAINT friendships_unique_pair
        UNIQUE (user_low_id, user_high_id)
);

CREATE INDEX IF NOT EXISTS idx_friendships_user_low
ON friendships (user_low_id);

CREATE INDEX IF NOT EXISTS idx_friendships_user_high
ON friendships (user_high_id);