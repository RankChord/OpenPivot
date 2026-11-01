ALTER TABLE user_sessions
    RENAME TO actor_sessions;

ALTER TABLE actor_sessions
    RENAME COLUMN user_id TO actor_id;

ALTER TABLE actor_sessions
    DROP CONSTRAINT user_sessions_user_id_fkey;

ALTER TABLE actor_sessions
    ADD CONSTRAINT actor_sessions_actor_id_fkey
    FOREIGN KEY (actor_id)
    REFERENCES actors(id)
    ON DELETE CASCADE;