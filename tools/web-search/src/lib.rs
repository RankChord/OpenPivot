use async_trait::async_trait;
use agent_tools::{Tool, ToolDefinition, ToolUseContext, ToolResult, ProgressUpdate};

#[derive(Debug)]
pub struct WebSearchTool {
    def: ToolDefinition,
}

impl WebSearchTool {
    pub fn new() -> Self {
        Self {
            def: ToolDefinition {
                name: "web_search".into(),
                description: "Search the web for information. Returns top results with titles, snippets, and URLs.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query"},
                        "max_results": {"type": "integer", "description": "Max results to return (default: 5)"}
                    },
                    "required": ["query"]
                }),
                toolset_name: "web".into(),
                aliases: vec!["search".into()],
                max_result_size: 10000,
            },
        }
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn definition(&self) -> &ToolDefinition { &self.def }

    async fn call(
        &self,
        input: serde_json::Value,
        _ctx: &ToolUseContext<'_>,
        _on_progress: &dyn Fn(ProgressUpdate),
    ) -> ToolResult {
        let query = match input["query"].as_str() {
            Some(q) => q,
            None => return ToolResult {
                ok: false,
                content: String::new(),
                error: Some("Missing required parameter: query".into()),
            },
        };

        let max_results = input["max_results"].as_u64().unwrap_or(5);

        let result = format!(
            "Search query: \"{}\"\nMax results: {}\n\nNote: This is a stub implementation.\nTo enable real search, configure a search API provider.",
            query, max_results
        );

        ToolResult { ok: true, content: result, error: None }
    }

    fn is_concurrency_safe(&self) -> bool { true }
    fn is_read_only(&self) -> bool { true }
}

#[allow(improper_ctypes_definitions)]
#[unsafe(no_mangle)]
pub extern "C" fn create_tool() -> Box<dyn Tool> {
    Box::new(WebSearchTool::new())
}
