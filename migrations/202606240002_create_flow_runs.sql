CREATE TABLE IF NOT EXISTS flow_runs (
    id UUID PRIMARY KEY,
    flow_id UUID NOT NULL REFERENCES flows(id) ON DELETE CASCADE,
    space_id UUID NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'running',
    started_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    current_task_id UUID,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,

    CONSTRAINT flow_runs_status_check
        CHECK (status IN ('running', 'waiting_action', 'completed', 'canceled', 'failed'))
);

CREATE INDEX IF NOT EXISTS idx_flow_runs_flow
ON flow_runs (flow_id);

CREATE INDEX IF NOT EXISTS idx_flow_runs_space
ON flow_runs (space_id);

CREATE INDEX IF NOT EXISTS idx_flow_runs_status
ON flow_runs (status);