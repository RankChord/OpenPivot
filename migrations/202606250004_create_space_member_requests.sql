-- 空间成员请求表
CREATE TABLE space_member_requests (
    id UUID PRIMARY KEY,
    space_id UUID NOT NULL
        REFERENCES spaces(id) ON DELETE CASCADE,
    requester_actor_id UUID NOT NULL
        REFERENCES actors(id) ON DELETE CASCADE,
    target_actor_id UUID NOT NULL
        REFERENCES actors(id) ON DELETE CASCADE,

    status TEXT NOT NULL DEFAULT 'pending',
    reviewer_actor_id UUID
        REFERENCES actors(id) ON DELETE SET NULL,
    reviewed_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT space_member_requests_status_check
        CHECK (status IN ('pending', 'approved', 'rejected')),

    CONSTRAINT space_member_requests_not_self
        CHECK (requester_actor_id <> target_actor_id)
);

-- 空间成员请求表的唯一索引，确保同一个空间中同一个目标用户只能有一个待处理的请求
CREATE UNIQUE INDEX space_member_requests_one_pending_target
ON space_member_requests (space_id, target_actor_id)
WHERE status = 'pending';

-- 创建索引以优化查询性能
CREATE INDEX idx_space_member_requests_space_status
ON space_member_requests (space_id, status);

-- 创建索引以优化查询性能，按目标用户和状态查询
CREATE INDEX idx_space_member_requests_target_status
ON space_member_requests (target_actor_id, status);