-- 将好友请求表中的请求方/接收方外键切换为 Actor
ALTER TABLE friend_requests
    DROP CONSTRAINT friend_requests_requester_id_fkey,
    DROP CONSTRAINT friend_requests_addressee_id_fkey;

-- 重新创建指向 actors 表的请求方和接收方外键
ALTER TABLE friend_requests
    ADD CONSTRAINT friend_requests_requester_id_fkey
        FOREIGN KEY (requester_id) REFERENCES actors(id) ON DELETE CASCADE,
    ADD CONSTRAINT friend_requests_addressee_id_fkey
        FOREIGN KEY (addressee_id) REFERENCES actors(id) ON DELETE CASCADE;


-- 删除 friendships 表中原来指向 users(id) 的外键
ALTER TABLE friendships
    DROP CONSTRAINT friendships_user_low_id_fkey,
    DROP CONSTRAINT friendships_user_high_id_fkey;

-- 将用户两端字段改为 Actor 语义
ALTER TABLE friendships
    RENAME COLUMN user_low_id TO actor_low_id,
    RENAME COLUMN user_high_id TO actor_high_id;

-- 删除使用旧字段名的约束
ALTER TABLE friendships
    DROP CONSTRAINT friendships_not_self,
    DROP CONSTRAINT friendships_ordered_pair,
    DROP CONSTRAINT friendships_unique_pair;

-- 重建 Actor 字段约束和外键
ALTER TABLE friendships
    ADD CONSTRAINT friendships_not_self
        CHECK (actor_low_id <> actor_high_id),
    ADD CONSTRAINT friendships_ordered_pair
        CHECK (actor_low_id < actor_high_id),
    ADD CONSTRAINT friendships_unique_pair
        UNIQUE (actor_low_id, actor_high_id),
    ADD CONSTRAINT friendships_actor_low_id_fkey
        FOREIGN KEY (actor_low_id) REFERENCES actors(id) ON DELETE CASCADE,
    ADD CONSTRAINT friendships_actor_high_id_fkey
        FOREIGN KEY (actor_high_id) REFERENCES actors(id) ON DELETE CASCADE;

-- 重命名索引
ALTER INDEX idx_friendships_user_low
    RENAME TO idx_friendships_actor_low;
ALTER INDEX idx_friendships_user_high
    RENAME TO idx_friendships_actor_high;

-- 删除 conversations 中原来指向 users 的外键
ALTER TABLE conversations
    DROP CONSTRAINT conversations_user_low_id_fkey,
    DROP CONSTRAINT conversations_user_high_id_fkey;

-- 将用户两端字段改为 Actor 语义
ALTER TABLE conversations
    RENAME COLUMN user_low_id TO actor_low_id,
    RENAME COLUMN user_high_id TO actor_high_id;

-- 删除旧字段约束
ALTER TABLE conversations
    DROP CONSTRAINT conversations_not_self,
    DROP CONSTRAINT conversations_ordered_pair,
    DROP CONSTRAINT conversations_unique_direct_pair;

-- 重建 Actor 字段约束和外键
ALTER TABLE conversations
    ADD CONSTRAINT conversations_not_self
        CHECK (actor_low_id <> actor_high_id),
    ADD CONSTRAINT conversations_ordered_pair
        CHECK (actor_low_id < actor_high_id),
    ADD CONSTRAINT conversations_unique_direct_pair
        UNIQUE (actor_low_id, actor_high_id),
    ADD CONSTRAINT conversations_actor_low_id_fkey
        FOREIGN KEY (actor_low_id) REFERENCES actors(id) ON DELETE CASCADE,
    ADD CONSTRAINT conversations_actor_high_id_fkey
        FOREIGN KEY (actor_high_id) REFERENCES actors(id) ON DELETE CASCADE;

-- 重命名 conversations 索引
ALTER INDEX idx_conversations_user_low
    RENAME TO idx_conversations_actor_low;

ALTER INDEX idx_conversations_user_high
    RENAME TO idx_conversations_actor_high;

-- 删除 messages.sender_id 原来指向 users 的外键
ALTER TABLE messages
    DROP CONSTRAINT messages_sender_id_fkey;

-- 重新指向 actors
ALTER TABLE messages
    ADD CONSTRAINT messages_sender_id_fkey
        FOREIGN KEY (sender_id) REFERENCES actors(id) ON DELETE CASCADE;


-- spaces
ALTER TABLE spaces
    DROP CONSTRAINT spaces_owner_id_fkey;

ALTER TABLE spaces
    ADD CONSTRAINT spaces_owner_id_fkey
        FOREIGN KEY (owner_id) REFERENCES actors(id) ON DELETE CASCADE;


-- space_members
ALTER TABLE space_members
    DROP CONSTRAINT space_members_user_id_fkey;

ALTER TABLE space_members
    RENAME COLUMN user_id TO actor_id;

ALTER TABLE space_members
    DROP CONSTRAINT space_members_unique_user;

ALTER TABLE space_members
    ADD CONSTRAINT space_members_actor_id_fkey
        FOREIGN KEY (actor_id) REFERENCES actors(id) ON DELETE CASCADE,
    ADD CONSTRAINT space_members_unique_actor
        UNIQUE (space_id, actor_id);

ALTER INDEX idx_space_members_user
    RENAME TO idx_space_members_actor;


-- space_messages
ALTER TABLE space_messages
    DROP CONSTRAINT space_messages_sender_id_fkey;

ALTER TABLE space_messages
    ADD CONSTRAINT space_messages_sender_id_fkey
        FOREIGN KEY (sender_id) REFERENCES actors(id) ON DELETE CASCADE;


-- flows
ALTER TABLE flows
    DROP CONSTRAINT flows_created_by_fkey;

ALTER TABLE flows
    ADD CONSTRAINT flows_created_by_fkey
        FOREIGN KEY (created_by) REFERENCES actors(id) ON DELETE CASCADE;


-- flow_runs
ALTER TABLE flow_runs
    DROP CONSTRAINT flow_runs_started_by_fkey;

ALTER TABLE flow_runs
    ADD CONSTRAINT flow_runs_started_by_fkey
        FOREIGN KEY (started_by) REFERENCES actors(id) ON DELETE CASCADE;


-- flow_tasks
ALTER TABLE flow_tasks
    DROP CONSTRAINT flow_tasks_assignee_id_fkey,
    DROP CONSTRAINT flow_tasks_completed_by_fkey;

ALTER TABLE flow_tasks
    ADD CONSTRAINT flow_tasks_assignee_id_fkey
        FOREIGN KEY (assignee_id) REFERENCES actors(id) ON DELETE CASCADE,
    ADD CONSTRAINT flow_tasks_completed_by_fkey
        FOREIGN KEY (completed_by) REFERENCES actors(id) ON DELETE SET NULL;


-- flow_events
ALTER TABLE flow_events
    DROP CONSTRAINT flow_events_actor_id_fkey;

ALTER TABLE flow_events
    ADD CONSTRAINT flow_events_actor_id_fkey
        FOREIGN KEY (actor_id) REFERENCES actors(id) ON DELETE SET NULL;


-- users 已不再被任何业务表引用
DROP TABLE users;

-- 将好友关系表改名为协作者关系表
ALTER TABLE friendships
    RENAME TO collaboratorships;

-- 将协作者申请表改为平台统一命名
ALTER TABLE friend_requests
    RENAME TO collaborator_requests;

ALTER INDEX idx_friend_requests_addressee_status
    RENAME TO idx_collaborator_requests_addressee_status;

ALTER INDEX idx_friend_requests_requester_status
    RENAME TO idx_collaborator_requests_requester_status;