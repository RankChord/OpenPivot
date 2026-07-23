CREATE TABLE IF NOT EXISTS flow_events (
    id UUID PRIMARY KEY,
    flow_run_id UUID NOT NULL REFERENCES flow_runs(id) ON DELETE CASCADE,
    space_id UUID NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    actor_id UUID REFERENCES users(id) ON DELETE SET NULL,
    payload JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT flow_events_type_check
        CHECK (event_type IN (
            'flow_started',
            'task_created',
            'task_completed',
            'space_notified',
            'flow_completed',
            'flow_canceled',
            'flow_failed'
        ))
);

CREATE INDEX IF NOT EXISTS idx_flow_events_run_created
ON flow_events (flow_run_id, created_at ASC);

CREATE INDEX IF NOT EXISTS idx_flow_events_space_created
ON flow_events (space_id, created_at ASC);