use agent_tools::{ProgressUpdate, Tool, ToolDefinition, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct StaticSearchResult {
    title: String,
    url: String,
    snippet: String,
}

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
    fn definition(&self) -> &ToolDefinition {
        &self.def
    }

    async fn call(
        &self,
        input: serde_json::Value,
        _ctx: &ToolUseContext<'_>,
        _on_progress: &dyn Fn(ProgressUpdate),
    ) -> ToolResult {
        let query = match input["query"].as_str() {
            Some(q) => q,
            None => {
                return ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some("Missing required parameter: query".into()),
                };
            }
        };

        if let Ok(raw_results) = std::env::var("AGENT_WEB_SEARCH_STATIC_RESULTS") {
            return match serde_json::from_str::<Vec<StaticSearchResult>>(&raw_results) {
                Ok(results) => ToolResult {
                    ok: true,
                    content: format_static_results(query, &results),
                    error: None,
                },
                Err(error) => ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(format!(
                        "Invalid AGENT_WEB_SEARCH_STATIC_RESULTS: {}",
                        error
                    )),
                },
            };
        }

        ToolResult {
            ok: false,
            content: String::new(),
            error: Some("web_search is not configured with a real search provider".into()),
        }
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }
    fn is_read_only(&self) -> bool {
        true
    }
}

fn format_static_results(query: &str, results: &[StaticSearchResult]) -> String {
    let mut output = format!("Search query: {}\n", query);
    for (index, result) in results.iter().enumerate() {
        output.push_str(&format!(
            "\n{}. {}\n{}\n{}\n",
            index + 1,
            result.title,
            result.url,
            result.snippet
        ));
    }
    output
}

#[allow(improper_ctypes_definitions)]
#[unsafe(no_mangle)]
pub extern "C" fn create_web_search_tool() -> Box<dyn Tool> {
    Box::new(WebSearchTool::new())
}
