use agent_tools::{ProgressUpdate, Tool, ToolDefinition, ToolResult, ToolUseContext};

#[derive(Debug)]
pub struct BrowserTool;

impl BrowserTool {
    pub fn new() -> Self {
        Self
    }

    async fn fetch_page(&self, url: &str) -> ToolResult {
        match reqwest::get(url).await {
            Ok(response) => match response.text().await {
                Ok(html) => {
                    let document = scraper::Html::parse_document(&html);
                    let selector = scraper::Selector::parse("body").unwrap();

                    let mut text_content = String::new();
                    for element in document.select(&selector) {
                        text_content.push_str(&element.text().collect::<Vec<_>>().join(" "));
                    }

                    // 截取内容以控制 token
                    let content = if text_content.len() > 8000 {
                        format!("{}...", &text_content[..8000])
                    } else {
                        text_content
                    };

                    ToolResult {
                        ok: true,
                        content,
                        error: None,
                    }
                }
                Err(e) => ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some(format!("Failed to read HTML: {}", e)),
                },
            },
            Err(e) => ToolResult {
                ok: false,
                content: String::new(),
                error: Some(format!("Network error: {}", e)),
            },
        }
    }
}

#[async_trait::async_trait]
impl Tool for BrowserTool {
    fn definition(&self) -> &ToolDefinition {
        static DEF: std::sync::OnceLock<ToolDefinition> = std::sync::OnceLock::new();
        DEF.get_or_init(|| ToolDefinition {
            name: "browse_web".into(),
            description: "Fetch a URL and extract readable text content from the page.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["fetch"],
                        "description": "The action to perform"
                    },
                    "url": {
                        "type": "string",
                        "description": "The URL to browse"
                    }
                },
                "required": ["action", "url"]
            }),
            toolset_name: "web".into(),
            aliases: vec!["search_web".into()],
            max_result_size: 20000,
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
            "fetch" => match input["url"].as_str() {
                Some(url) => self.fetch_page(url).await,
                None => ToolResult {
                    ok: false,
                    content: String::new(),
                    error: Some("Missing URL".into()),
                },
            },
            _ => ToolResult {
                ok: false,
                content: String::new(),
                error: Some("Unknown action".into()),
            },
        }
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }
    fn is_read_only(&self) -> bool {
        true
    }
}

#[allow(improper_ctypes_definitions)]
#[unsafe(no_mangle)]
pub extern "C" fn create_browser_tool() -> Box<dyn Tool> {
    Box::new(BrowserTool::new())
}
