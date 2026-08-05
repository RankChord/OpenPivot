use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::models::flow::{
    Flow,
    FlowRun,
    FlowRunStatus,
    FlowTask,
    FlowTaskStatus,
    FlowEvent,
    FlowEventType,
};

#[derive(sqlx::FromRow)]
struct FlowRow {
    id: Uuid,
    space_id: Uuid,
    name: String,
    description: Option<String>,
    created_by: Uuid,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<FlowRow> for Flow {
    fn from(row: FlowRow) -> Self {
        Flow {
            id: row.id,
            space_id: row.space_id,
            name: row.name,
            description: row.description,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

pub async fn create_flow(
    pool: &PgPool,
    space_id: Uuid,
    created_by: Uuid,
    name: &str,
    description: Option<&str>,
) -> Result<Flow, sqlx::Error> {
    let flow_id = Uuid::now_v7();
    let row = sqlx::query_as::<_, FlowRow>(
        r#"
        INSERT INTO flows (id, space_id, name, description, created_by)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING
            id,
            space_id,
            name,
            description,
            created_by,
            created_at,
            updated_at
        "#,
    )
    .bind(flow_id)
    .bind(space_id)
    .bind(name)
    .bind(description)
    .bind(created_by)
    .fetch_one(pool)
    .await?;

    Ok(Flow::from(row))
}

pub async fn list_flows_by_space(
    pool: &PgPool,
    space_id: Uuid,
) -> Result<Vec<Flow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, FlowRow>(
        r#"
        SELECT
            id,
            space_id,
            name,
            description,
            created_by,
            created_at,
            updated_at
        FROM flows
        WHERE space_id = $1
        ORDER BY updated_at DESC
        "#,
    )
    .bind(space_id)
    .fetch_all(pool)
    .await?;

    let flows = rows
        .into_iter()
        .map(Flow::from)
        .collect();

    Ok(flows)
}

#[derive(sqlx::FromRow)]
struct FlowRunRow {
    id: Uuid,
    flow_id: Uuid,
    space_id: Uuid,
    status: String,
    started_by: Uuid,
    current_task_id: Option<Uuid>,
    started_at: OffsetDateTime,
    completed_at: Option<OffsetDateTime>,
}

impl TryFrom<FlowRunRow> for FlowRun {
    type Error = sqlx::Error;

    fn try_from(row: FlowRunRow) -> Result<Self, Self::Error> {
        let status = FlowRunStatus::try_from(row.status.as_str())
            .map_err(|_| sqlx::Error::Decode("invalid flow run status".into()))?;

        Ok(FlowRun {
            id: row.id,
            flow_id: row.flow_id,
            space_id: row.space_id,
            status,
            started_by: row.started_by,
            current_task_id: row.current_task_id,
            started_at: row.started_at,
            completed_at: row.completed_at,
        })
    }
}

pub async fn create_flow_run(
    pool: &PgPool,
    flow_id: Uuid,
    space_id: Uuid,
    started_by: Uuid,
) -> Result<FlowRun, sqlx::Error> {
    let flow_run_id = Uuid::now_v7();
    let row = sqlx::query_as::<_, FlowRunRow>(
        r#"
        INSERT INTO flow_runs (id, flow_id, space_id, status, started_by)
        VALUES ($1, $2, $3, 'running', $4)
        RETURNING id, flow_id, space_id, status, started_by, current_task_id, started_at, completed_at
        "#,
    )
    .bind(flow_run_id)
    .bind(flow_id)
    .bind(space_id)
    .bind(started_by)
    .fetch_one(pool)
    .await?;

    FlowRun::try_from(row)
}

#[derive(sqlx::FromRow)]
struct FlowTaskRow {
    id: Uuid,
    flow_run_id: Uuid,
    space_id: Uuid,
    assignee_id: Uuid,
    title: String,
    description: Option<String>,
    status: String,
    result: Option<String>,
    created_at: OffsetDateTime,
    completed_at: Option<OffsetDateTime>,
    completed_by: Option<Uuid>,
}

impl TryFrom<FlowTaskRow> for FlowTask {
    type Error = sqlx::Error;

    fn try_from(row: FlowTaskRow) -> Result<Self, Self::Error> {
        let status = FlowTaskStatus::try_from(row.status.as_str())
            .map_err(|_| sqlx::Error::Decode("invalid flow task status".into()))?;

        Ok(FlowTask {
            id: row.id,
            flow_run_id: row.flow_run_id,
            space_id: row.space_id,
            assignee_id: row.assignee_id,
            title: row.title,
            description: row.description,
            status,
            result: row.result,
            created_at: row.created_at,
            completed_at: row.completed_at,
            completed_by: row.completed_by,
        })
    }
}

pub async fn create_flow_task(
    pool: &PgPool,
    flow_run_id: Uuid,
    space_id: Uuid,
    assignee_id: Uuid,
    title: &str,
    description: Option<&str>,
) -> Result<FlowTask, sqlx::Error> {
    let task_id = Uuid::now_v7();
    let row = sqlx::query_as::<_, FlowTaskRow>(
        r#"
        INSERT INTO flow_tasks (
            id,
            flow_run_id,
            space_id,
            assignee_id,
            title,
            description,
            status
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'pending')
        RETURNING
            id,
            flow_run_id,
            space_id,
            assignee_id,
            title,
            description,
            status,
            result,
            created_at,
            completed_at,
            completed_by
        "#,
    )
    .bind(task_id)
    .bind(flow_run_id)
    .bind(space_id)
    .bind(assignee_id)
    .bind(title)
    .bind(description)
    .fetch_one(pool)
    .await?;

    FlowTask::try_from(row)
}

pub async fn set_flow_run_current_task(
    pool: &PgPool,
    flow_run_id: Uuid,
    task_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE flow_runs
        SET current_task_id = $2,
            status = 'waiting_action'
        WHERE id = $1
        "#,
    )
    .bind(flow_run_id)
    .bind(task_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn append_flow_event(
    pool: &PgPool,
    flow_run_id: Uuid,
    space_id: Uuid,
    event_type: FlowEventType,
    actor_id: Option<Uuid>,
    payload: &str,
) -> Result<FlowEvent, sqlx::Error> {
    let event_id = Uuid::now_v7();
    let event = sqlx::query_as::<_, FlowEvent>(
        r#"
        INSERT INTO flow_events (
            id,
            flow_run_id,
            space_id,
            event_type,
            actor_id,
            payload
        )
        VALUES ($1, $2, $3, $4, $5, $6::JSONB)
        RETURNING
            id,
            flow_run_id,
            space_id,
            event_type,
            actor_id,
            payload::TEXT AS payload,
            created_at
        "#,
    )
    .bind(event_id)
    .bind(flow_run_id)
    .bind(space_id)
    .bind(event_type.as_str())
    .bind(actor_id)
    .bind(payload)
    .fetch_one(pool)
    .await?;

    Ok(event)
}

pub async fn start_manual_flow_run(
    pool: &PgPool,
    flow_id: Uuid,
    space_id: Uuid,
    started_by: Uuid,
    assignee_id: Uuid,
    task_title: &str,
    task_description: Option<&str>,
) -> Result<(FlowRun, FlowTask), sqlx::Error> {
    let run = create_flow_run(
        pool,
        flow_id,
        space_id,
        started_by,
    )
    .await?;

    let task = create_flow_task(
        pool,
        run.id,
        space_id,
        assignee_id,
        task_title,
        task_description,
    )
    .await?;

    set_flow_run_current_task(
        pool,
        run.id,
        task.id,
    )
    .await?;

    append_flow_event(
        pool,
        run.id,
        space_id,
        FlowEventType::FlowStarted,
        Some(started_by),
        "{}",
    )
    .await?;

    append_flow_event(
        pool,
        run.id,
        space_id,
        FlowEventType::TaskCreated,
        Some(started_by),
        "{}",
    )
    .await?;

    Ok((run, task))
}

pub async fn complete_flow_task(
    pool: &PgPool,
    task_id: Uuid,
    completed_by: Uuid,
    result: &str,
) -> Result<FlowTask, sqlx::Error> {
    let row = sqlx::query_as::<_, FlowTaskRow>(
        r#"
        UPDATE flow_tasks
        SET status = 'completed',
            result = $3,
            completed_by = $2,
            completed_at = NOW()
        WHERE id = $1
          AND assignee_id = $2
          AND status = 'pending'
        RETURNING
            id,
            flow_run_id,
            space_id,
            assignee_id,
            title,
            description,
            status,
            result,
            created_at,
            completed_at,
            completed_by
        "#,
    )
    .bind(task_id)
    .bind(completed_by)
    .bind(result)
    .fetch_one(pool)
    .await?;

    FlowTask::try_from(row)
}

pub async fn complete_flow_run(
    pool: &PgPool,
    flow_run_id: Uuid,
) -> Result<FlowRun, sqlx::Error> {
    let row = sqlx::query_as::<_, FlowRunRow>(
        r#"
        UPDATE flow_runs
        SET status = 'completed',
            completed_at = NOW()
        WHERE id = $1
          AND status = 'waiting_action'
        RETURNING
            id,
            flow_id,
            space_id,
            status,
            started_by,
            current_task_id,
            started_at,
            completed_at
        "#,
    )
    .bind(flow_run_id)
    .fetch_one(pool)
    .await?;

    FlowRun::try_from(row)
}