CREATE TABLE IF NOT EXISTS friend_requests (
    id BIGSERIAL PRIMARY KEY,
    requester_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    addressee_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending',
    message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT friend_requests_not_self
        CHECK (requester_id <> addressee_id),

    CONSTRAINT friend_requests_unique_pair
        UNIQUE (requester_id, addressee_id)
);

CREATE INDEX IF NOT EXISTS idx_friend_requests_addressee_status
ON friend_requests (addressee_id, status);

CREATE INDEX IF NOT EXISTS idx_friend_requests_requester_status
ON friend_requests (requester_id, status);