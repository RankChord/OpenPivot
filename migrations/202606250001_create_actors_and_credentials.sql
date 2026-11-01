-- 创建 actors 表
CREATE TABLE actors (
    id UUID PRIMARY KEY,
    type TEXT NOT NULL DEFAULT 'human',
    display_name TEXT NOT NULL,
    avatar TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    profile_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT actors_type_check
        CHECK (type IN ('human', 'agent'))
);

-- 说明 actors.type 字段的用途
COMMENT ON COLUMN actors.type IS
'Registration label only. Business logic must not branch on actor type.';

-- 创建 credentials 表
CREATE TABLE credentials (
    actor_id UUID PRIMARY KEY REFERENCES actors(id) ON DELETE CASCADE,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    last_login_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);  

-- 迁移现有的 users 数据到 actors 表
INSERT INTO actors (
    id, type, display_name, avatar, status,
    profile_json, created_at, updated_at
)
SELECT
    id,
    'human',
    nickname,
    avatar_url,
    status,
    '{}'::jsonb,
    created_at,
    updated_at
FROM users;

-- 迁移现有的 users 数据到 credentials 表
INSERT INTO credentials (
    actor_id, username, password_hash, last_login_at
)
SELECT
    id,
    username,
    password_hash,
    last_login_at
FROM users;