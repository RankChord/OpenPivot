CREATE TABLE IF NOT EXISTS flow_tasks (
    id UUID PRIMARY KEY,
    flow_run_id UUID NOT NULL REFERENCES flow_runs(id) ON DELETE CASCADE,
    space_id UUID NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    assignee_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    result TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    completed_by UUID REFERENCES users(id) ON DELETE SET NULL,

    CONSTRAINT flow_tasks_status_check
        CHECK (status IN ('pending', 'completed', 'canceled'))
);

CREATE INDEX IF NOT EXISTS idx_flow_tasks_assignee_status
ON flow_tasks (assignee_id, status);

CREATE INDEX IF NOT EXISTS idx_flow_tasks_run
ON flow_tasks (flow_run_id);

CREATE INDEX IF NOT EXISTS idx_flow_tasks_space
ON flow_tasks (space_id);