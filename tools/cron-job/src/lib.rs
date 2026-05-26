use agent_cron::{CronStore, MissedRunPolicy, NewCronJob, PayloadKind, Schedule};
use agent_tools::{ProgressUpdate, Tool, ToolDefinition, ToolResult, ToolUseContext};
use std::path::PathBuf;

const AUTOMATION_DB: &str = ".agent/automation.db";

#[derive(Debug)]
pub struct CronJobTool;

impl CronJobTool {
    pub fn new() -> Self {
        Self
    }

    fn automation_db() -> PathBuf {
        dirs::home_dir().unwrap_or_default().join(AUTOMATION_DB)
    }

    async fn store() -> Result<CronStore, String> {
        CronStore::new(&Self::automation_db())
            .await
            .map_err(|e| format!("Failed to open cron store: {}", e))
    }

    async fn handle_create(input: serde_json::Value) -> ToolResult {
        let schedule_str = match input["schedule"].as_str() {
            Some(s) => s.to_string(),
            None => {
                return ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(
                        "Missing 'schedule' field (e.g., '0 10 * * *' or 'every 2h')".into(),
                    ),
                };
            }
        };
        let command = match input["command"].as_str() {
            Some(c) => c.to_string(),
            None => {
                return ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some("Missing 'command' field (the command/script to run)".into()),
                };
            }
        };
        let name = input["name"].as_str().unwrap_or("Cron Task").to_string();
        let description = input["description"]
            .as_str()
            .unwrap_or(&schedule_str)
            .to_string();

        let schedule = match Schedule::parse(&schedule_str) {
            Ok(s) => s,
            Err(e) => {
                return ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(format!("Invalid schedule: {}", e)),
                };
            }
        };

        let job = NewCronJob {
            name,
            description,
            schedule,
            payload_kind: PayloadKind::AgentPrompt,
            payload_json: serde_json::json!({ "prompt": command }),
            enabled: true,
            missed_run_policy: MissedRunPolicy::RunOnce,
        };

        match Self::store().await {
            Ok(store) => match store.create_job(job).await {
                Ok(job) => ToolResult {
                    ok: true,
                    content: format!(
                        "Cron job '{}' created (ID: {}, next run: {})",
                        job.name,
                        job.id,
                        job.next_run_at
                            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_else(|| "N/A".into())
                    ),
                    error: None,
                },
                Err(e) => ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(format!("Failed to create cron job: {}", e)),
                },
            },
            Err(e) => ToolResult {
                ok: false,
                content: String::new(),
                error: Some(e),
            },
        }
    }

    async fn handle_list() -> ToolResult {
        let jobs = match Self::store().await {
            Ok(store) => match store.list_jobs().await {
                Ok(jobs) => jobs,
                Err(e) => {
                    return ToolResult {
                        ok: false,
                        content: String::new(),
                        error: Some(format!("Failed to list cron jobs: {}", e)),
                    };
                }
            },
            Err(e) => {
                return ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(e),
                };
            }
        };

        if jobs.is_empty() {
            return ToolResult {
                ok: true,
                content: "No cron jobs found.".into(),
                error: None,
            };
        }
        let mut lines = String::from("Active Cron Jobs:\n\n");
        for j in &jobs {
            let status = if j.enabled { "enabled" } else { "disabled" };
            let next = j
                .next_run_at
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "N/A".into());
            lines.push_str(&format!(
                "  • {} (ID: {})\n    Schedule: {}\n    Next: {}\n    Status: {}\n\n",
                j.name, j.id, j.description, next, status
            ));
        }
        ToolResult {
            ok: true,
            content: lines,
            error: None,
        }
    }

    async fn handle_delete(input: serde_json::Value) -> ToolResult {
        let job_id = match input["id"].as_str() {
            Some(id) => id.to_string(),
            None => {
                return ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some("Missing 'id' field".into()),
                };
            }
        };
        match Self::store().await {
            Ok(store) => match store.delete_job(&job_id).await {
                Ok(true) => ToolResult {
                    ok: true,
                    content: format!("Cron job '{}' deleted", job_id),
                    error: None,
                },
                Ok(false) => ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(format!("Job '{}' not found", job_id)),
                },
                Err(e) => ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(format!("Failed to delete cron job: {}", e)),
                },
            },
            Err(e) => ToolResult {
                ok: false,
                content: String::new(),
                error: Some(e),
            },
        }
    }

    async fn handle_pause(input: serde_json::Value) -> ToolResult {
        let job_id = match input["id"].as_str() {
            Some(id) => id.to_string(),
            None => {
                return ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some("Missing 'id' field".into()),
                };
            }
        };
        match Self::store().await {
            Ok(store) => match store.load_job(&job_id).await {
                Ok(Some(_)) => match store.set_enabled(&job_id, false).await {
                    Ok(()) => ToolResult {
                        ok: true,
                        content: format!("Cron job '{}' paused", job_id),
                        error: None,
                    },
                    Err(e) => ToolResult {
                        ok: false,
                        content: String::new(),
                        error: Some(format!("Failed to pause cron job: {}", e)),
                    },
                },
                Ok(None) => ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(format!("Job '{}' not found", job_id)),
                },
                Err(e) => ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(format!("Failed to load cron job: {}", e)),
                },
            },
            Err(e) => ToolResult {
                ok: false,
                content: String::new(),
                error: Some(e),
            },
        }
    }

    async fn handle_resume(input: serde_json::Value) -> ToolResult {
        let job_id = match input["id"].as_str() {
            Some(id) => id.to_string(),
            None => {
                return ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some("Missing 'id' field".into()),
                };
            }
        };
        match Self::store().await {
            Ok(store) => match store.load_job(&job_id).await {
                Ok(Some(_)) => match store.set_enabled(&job_id, true).await {
                    Ok(()) => ToolResult {
                        ok: true,
                        content: format!("Cron job '{}' resumed", job_id),
                        error: None,
                    },
                    Err(e) => ToolResult {
                        ok: false,
                        content: String::new(),
                        error: Some(format!("Failed to resume cron job: {}", e)),
                    },
                },
                Ok(None) => ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(format!("Job '{}' not found", job_id)),
                },
                Err(e) => ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(format!("Failed to load cron job: {}", e)),
                },
            },
            Err(e) => ToolResult {
                ok: false,
                content: String::new(),
                error: Some(e),
            },
        }
    }
}

#[async_trait::async_trait]
impl Tool for CronJobTool {
    fn definition(&self) -> &ToolDefinition {
        static DEF: std::sync::OnceLock<ToolDefinition> = std::sync::OnceLock::new();
        DEF.get_or_init(|| ToolDefinition {
            name: "cron_job".into(),
            description: "Manage cron jobs. Create, list, delete, pause, or resume scheduled tasks. Supports cron expressions ('0 10 * * *'), durations ('30m', '2h'), and ISO timestamps.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["create", "list", "delete", "pause", "resume"],
                        "description": "The action to perform"
                    },
                    "schedule": {
                        "type": "string",
                        "description": "Schedule format: cron expression ('0 10 * * *'), duration ('30m','2h','1d'), or ISO timestamp"
                    },
                    "command": {
                        "type": "string",
                        "description": "Command or script to execute"
                    },
                    "name": {
                        "type": "string",
                        "description": "Job name/title"
                    },
                    "id": {
                        "type": "string",
                        "description": "Job ID (required for delete/pause/resume)"
                    }
                },
                "required": ["action"]
            }),
            toolset_name: "cron".into(),
            aliases: vec!["schedule_task".into(), "cron".into()],
            max_result_size: 5000,
        })
    }

    async fn call(
        &self,
        input: serde_json::Value,
        _ctx: &ToolUseContext<'_>,
        _on_progress: &dyn Fn(ProgressUpdate),
    ) -> ToolResult {
        let action = input["action"].as_str().unwrap_or("");
        match action {
            "create" => Self::handle_create(input).await,
            "list" => Self::handle_list().await,
            "delete" => Self::handle_delete(input).await,
            "pause" => Self::handle_pause(input).await,
            "resume" => Self::handle_resume(input).await,
            _ => ToolResult {
                ok: false,
                content: String::new(),
                error: Some(format!(
                    "Unknown action: '{}'. Supported: create, list, delete, pause, resume",
                    action
                )),
            },
        }
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }
}

#[allow(improper_ctypes_definitions)]
#[unsafe(no_mangle)]
pub extern "C" fn create_tool() -> Box<dyn Tool> {
    Box::new(CronJobTool::new())
}
