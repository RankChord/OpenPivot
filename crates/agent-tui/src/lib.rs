use std::collections::VecDeque;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

// Agent Brain
use agent_config::AgentConfig;
use agent_core::{
    Agent, AgentEvent, AgentInput, AgentOutput, IterationBudget, callback_event_sink,
};
use agent_llm::{LlmClient, LlmClientConfig, LlmProviderType};
use agent_tools::ToolRegistry;

// Tools
use cron_job::CronJobTool;
use read_file::ReadFileTool;
use shell_tool::ShellTool;
use todo_tool::TodoTool;
use web_search::WebSearchTool;
use write_file::WriteFileTool;

static RUNNING: AtomicBool = AtomicBool::new(true);

// --- Application State ---

#[derive(Clone)]
struct Message {
    role: String,
    content: String,
}

pub fn event_to_timeline_message(event: &AgentEvent) -> Option<(String, String)> {
    match event {
        AgentEvent::UserMessage { content } => Some(("User".into(), content.clone())),
        AgentEvent::ToolCall { name, .. } => Some(("Tool".into(), format!("Running {}", name))),
        AgentEvent::ToolResult {
            ok, content, error, ..
        } => {
            if *ok {
                Some(("Tool".into(), format!("Result: {}", content)))
            } else {
                Some((
                    "Tool".into(),
                    format!("Failed: {}", error.as_deref().unwrap_or("unknown error")),
                ))
            }
        }
        AgentEvent::Final { content } => Some(("Agent".into(), content.clone())),
        AgentEvent::BudgetExhausted { reason } => {
            Some(("Agent".into(), format!("[Budget exhausted] {}", reason)))
        }
    }
}

pub struct AppState {
    messages: VecDeque<Message>,
    current_input: String,
    cursor_position: usize,
    status_bar: String,
    is_thinking: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            messages: VecDeque::new(),
            current_input: String::new(),
            cursor_position: 0,
            status_bar: "Ready. Press Esc to quit.".to_string(),
            is_thinking: false,
        }
    }

    pub fn handle_input(
        &mut self,
        _key: KeyEventKind,
        code: KeyCode,
        modifiers: KeyModifiers,
    ) -> Option<String> {
        if self.is_thinking {
            return None;
        }

        match code {
            KeyCode::Char(c) => {
                if modifiers.contains(KeyModifiers::CONTROL) && c == 'u' {
                    // Ctrl+U to clear input
                    self.current_input.clear();
                    self.cursor_position = 0;
                } else if modifiers.is_empty() || modifiers.contains(KeyModifiers::SHIFT) {
                    let byte_index =
                        byte_index_for_char_position(&self.current_input, self.cursor_position);
                    self.current_input.insert(byte_index, c);
                    self.cursor_position += 1;
                }
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    let byte_index =
                        byte_index_for_char_position(&self.current_input, self.cursor_position - 1);
                    self.current_input.remove(byte_index);
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_position < self.current_input.chars().count() {
                    self.cursor_position += 1;
                }
            }
            KeyCode::Home => self.cursor_position = 0,
            KeyCode::End => self.cursor_position = self.current_input.chars().count(),
            KeyCode::Enter => {
                let input_text = self.current_input.trim().to_string();
                if !input_text.is_empty() {
                    self.messages.push_back(Message {
                        role: "User".into(),
                        content: input_text.clone(),
                    });
                    self.current_input.clear();
                    self.cursor_position = 0;
                    return Some(input_text);
                }
            }
            _ => {}
        }
        None
    }
}

fn byte_index_for_char_position(input: &str, char_position: usize) -> usize {
    input
        .char_indices()
        .nth(char_position)
        .map(|(index, _)| index)
        .unwrap_or(input.len())
}

// --- Rendering ---

fn render_messages(messages: &VecDeque<Message>) -> Text<'static> {
    let mut text = Vec::new();

    for msg in messages {
        let role_style = match msg.role.as_str() {
            "User" => Style::default().fg(Color::Cyan).bold(),
            "Agent" => Style::default().fg(Color::Green),
            "sys" => Style::default().fg(Color::Yellow),
            _ => Style::default(),
        };

        text.push(Line::from(Span::styled(
            format!("[{}] ", msg.role),
            role_style,
        )));

        // Simple Markdown simulation for code blocks
        let lines: Vec<&str> = msg.content.split('\n').collect();
        let mut in_code_block = false;

        for line in lines {
            if line.trim() == "```" {
                in_code_block = !in_code_block;
            }

            let style = if in_code_block {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            } else {
                Style::default()
            };

            text.push(Line::from(Span::styled(line.to_string(), style)));
        }
        text.push(Line::from(""));
    }

    Text::from(text)
}

fn ui(frame: &mut Frame, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(5),    // Chat Area
            Constraint::Length(3), // Input Area
        ])
        .split(frame.area());

    // 1. Header
    let header =
        Paragraph::new(" 🦞 One-Person Company Agent System | Model: qwen3.6-pro | HTTPS: Secure")
            .style(Style::default().fg(Color::Black).bg(Color::Yellow))
            .bold();
    frame.render_widget(header, chunks[0]);

    // 2. Chat Area
    let message_text = render_messages(&app.messages);
    let chat_area = Paragraph::new(message_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Conversation History"),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(chat_area, chunks[1]);

    // 3. Input Area
    let status_text = if app.is_thinking {
        " Thinking..."
    } else {
        &app.status_bar
    };
    let input_style = if app.is_thinking {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::Gray)
    };

    let prompt = " > ";
    let text = format!("{}{}", prompt, app.current_input);

    let input_box = Paragraph::new(text).style(input_style).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Status {} ", status_text))
            .title_style(Style::default().fg(if app.is_thinking {
                Color::Yellow
            } else {
                Color::Green
            })),
    );

    frame.render_widget(input_box, chunks[2]);

    // Cursor rendering
    if !app.is_thinking {
        let x = chunks[2].x + 1 + prompt.chars().count() as u16;
        let y = chunks[2].y + 1 + 1; // Header border is 1
        frame.set_cursor_position((
            std::cmp::min(x + app.cursor_position as u16, chunks[2].width - 2),
            y,
        ));
    }
}

// --- Main Logic ---

pub fn run() -> io::Result<()> {
    tracing::info!("starting TUI");
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let (prompt_tx, prompt_rx) = mpsc::channel::<String>();
    let (response_tx, response_rx) = mpsc::channel::<Message>();

    let mut app = AppState::new();

    // Background Thread
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to start Runtime");
        let agent = rt.block_on(init_agent());

        loop {
            if let Ok(user_message) = prompt_rx.recv() {
                let mut input = AgentInput {
                    user_message,
                    session_id: "tui-session".to_string(),
                    system_prompt:
                        "You are an expert coding assistant. Format your code blocks with ```."
                            .to_string(),
                    history: vec![],
                    workspace_dir: std::env::current_dir().unwrap_or_default(),
                };

                let event_tx = response_tx.clone();
                let mut sink = callback_event_sink(move |event| {
                    if let Some((role, content)) = event_to_timeline_message(&event) {
                        let _ = event_tx.send(Message { role, content });
                    }
                });

                let result = rt.block_on(async {
                    tracing::info!(session = %input.session_id, "running TUI agent request");
                    agent.run_with_events(&mut input, &mut sink).await
                });

                let entry = match result {
                    Ok(AgentOutput::Final(_)) => continue,
                    Ok(AgentOutput::BudgetExhausted(msg)) => Message {
                        role: "Agent".into(),
                        content: format!("[Budget Err] {}", msg),
                    },
                    Ok(AgentOutput::Cancelled) => Message {
                        role: "Agent".into(),
                        content: "[Cancelled]".into(),
                    },
                    Err(e) => Message {
                        role: "Agent".into(),
                        content: format!("[Error] {}", e),
                    },
                };
                let _ = response_tx.send(entry);
            }
        }
    });

    let res = run_app(&mut term, &mut app, prompt_tx, response_rx);

    disable_raw_mode()?;
    execute!(
        term.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    term.show_cursor()?;
    res
}

async fn init_agent() -> Agent {
    // 修复：强制读取用户真实的配置文件
    let config_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".agent")
        .join("config.toml");

    let config = if config_path.exists() {
        AgentConfig::from_file(&config_path).unwrap_or_default()
    } else {
        AgentConfig::default()
    };

    let api_key = config
        .model
        .api_key
        .clone()
        .or_else(|| std::env::var("AGENT_API_KEY").ok())
        .unwrap_or("no-key-provided".into());
    let base_url = config
        .model
        .base_url
        .clone()
        .unwrap_or("https://api.openai.com/v1".into());

    let client = LlmClient::new(LlmClientConfig {
        provider: match config.model.provider.as_str() {
            "anthropic" => LlmProviderType::Anthropic,
            "google" => LlmProviderType::Google,
            _ => LlmProviderType::OpenAI,
        },
        base_url,
        api_key,
        model: config.model.name,
        max_tokens: config.model.max_tokens,
        temperature: config.model.temperature,
    });

    let registry = ToolRegistry::new();

    // 修复：直接 await，不要再创建新的 Runtime 或 block_on
    registry.register(Box::new(ReadFileTool::new())).await;
    registry.register(Box::new(WriteFileTool::new())).await;
    registry.register(Box::new(ShellTool::new())).await;
    registry.register(Box::new(TodoTool::new())).await;
    registry.register(Box::new(WebSearchTool::new())).await;
    registry.register(Box::new(CronJobTool::new())).await;

    Agent::new(
        client,
        registry,
        IterationBudget::new(config.agent.max_iterations),
    )
    .with_permissions(config.permissions.clone())
}

fn run_app(
    term: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut AppState,
    prompt_tx: mpsc::Sender<String>,
    response_rx: mpsc::Receiver<Message>,
) -> io::Result<()> {
    loop {
        term.draw(|f| ui(f, app))?;

        if event::poll(std::time::Duration::from_millis(20))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if key.code == KeyCode::Esc {
                    RUNNING.store(false, Ordering::SeqCst);
                    break;
                }

                app.handle_input(key.kind, key.code, key.modifiers)
                    .map(|text| {
                        app.is_thinking = true;
                        app.status_bar = "Sending request to agent...".into();
                        let _ = prompt_tx.send(text);
                    });
            }
        }

        // Check for response
        if let Ok(msg) = response_rx.try_recv() {
            app.messages.push_back(msg);
            app.is_thinking = false;
            app.status_bar = "Ready. Press Esc to quit.".into();
        }
    }
    Ok(())
}
