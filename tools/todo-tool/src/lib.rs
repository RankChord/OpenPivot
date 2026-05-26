use agent_tools::{ProgressUpdate, Tool, ToolDefinition, ToolResult, ToolUseContext};
use async_trait::async_trait;
use std::sync::Mutex;

#[derive(Debug)]
pub struct TodoTool {
    def: ToolDefinition,
    todos: Mutex<Vec<(String, bool)>>,
}

impl TodoTool {
    pub fn new() -> Self {
        Self {
            def: ToolDefinition {
                name: "todo".into(),
                description: "Manage a todo list. Actions: add, complete, list, clear".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "action": {"type": "string", "enum": ["add", "complete", "list", "clear"], "description": "Action to perform"},
                        "task": {"type": "string", "description": "Task description (for add/complete)"}
                    },
                    "required": ["action"]
                }),
                toolset_name: "todo".into(),
                aliases: vec![],
                max_result_size: 5000,
            },
            todos: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl Tool for TodoTool {
    fn definition(&self) -> &ToolDefinition {
        &self.def
    }

    async fn call(
        &self,
        input: serde_json::Value,
        _ctx: &ToolUseContext<'_>,
        _on_progress: &dyn Fn(ProgressUpdate),
    ) -> ToolResult {
        let action = input["action"].as_str().unwrap_or("");
        let mut todos = self.todos.lock().unwrap();

        match action {
            "add" => {
                let task = input["task"].as_str().unwrap_or("unnamed task");
                todos.push((task.to_string(), false));
                ToolResult {
                    ok: true,
                    content: format!("Added: {}", task),
                    error: None,
                }
            }
            "complete" => {
                let task = input["task"].as_str().unwrap_or("");
                for (t, done) in todos.iter_mut() {
                    if t.contains(task) {
                        *done = true;
                        return ToolResult {
                            ok: true,
                            content: format!("Completed: {}", t),
                            error: None,
                        };
                    }
                }
                ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(format!("Task not found: {}", task)),
                }
            }
            "list" => {
                if todos.is_empty() {
                    ToolResult {
                        ok: true,
                        content: "No todos".into(),
                        error: None,
                    }
                } else {
                    let content = todos
                        .iter()
                        .enumerate()
                        .map(|(i, (t, d))| {
                            format!("{}. {} {}", i + 1, if *d { "[x]" } else { "[ ]" }, t)
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    ToolResult {
                        ok: true,
                        content,
                        error: None,
                    }
                }
            }
            "clear" => {
                todos.clear();
                ToolResult {
                    ok: true,
                    content: "Cleared all todos".into(),
                    error: None,
                }
            }
            _ => ToolResult {
                ok: false,
                content: String::new(),
                error: Some(format!("Unknown action: {}", action)),
            },
        }
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }
}

#[allow(improper_ctypes_definitions)]
#[unsafe(no_mangle)]
pub extern "C" fn create_todo_tool() -> Box<dyn Tool> {
    Box::new(TodoTool::new())
}
