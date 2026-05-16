use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent},
    DefaultTerminal,
};
use crate::terminal;
use crate::widgets;

pub struct App {
    pub messages: Vec<String>,
    pub current_input: String,
    pub cursor_visible: bool,
    pub running: bool,
    pub agent_thinking: bool,
    pub thinking_text: String,
}

impl App {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            current_input: String::new(),
            cursor_visible: true,
            running: true,
            agent_thinking: false,
            thinking_text: String::new(),
        }
    }

    pub fn add_message(&mut self, msg: &str) {
        self.messages.push(msg.to_string());
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                if !self.current_input.is_empty() {
                    self.messages.push(format!("You: {}", self.current_input));
                    self.current_input.clear();
                    self.agent_thinking = true;
                    self.thinking_text = "Agent is thinking...".into();
                }
            }
            KeyCode::Backspace => {
                self.current_input.pop();
            }
            KeyCode::Char(c) => {
                self.current_input.push(c);
            }
            KeyCode::Esc => {
                self.running = false;
            }
            _ => {}
        }
    }

    pub fn set_agent_response(&mut self, response: &str) {
        self.agent_thinking = false;
        self.messages.push(format!("Agent: {}", response));
    }

    pub fn handle_events(&mut self) -> bool {
        if event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
            if let Event::Key(KeyEvent { code, .. }) = event::read().unwrap() {
                self.handle_key(code);
            }
        }
        self.running
    }

    pub fn draw(&self, terminal: &mut DefaultTerminal) -> Result<(), std::io::Error> {
        terminal.draw(|frame| {
            if self.agent_thinking {
                widgets::render_thinking(frame, &self.thinking_text);
            } else {
                widgets::render_chat_area(
                    frame,
                    &self.messages,
                    &self.current_input,
                    self.cursor_visible,
                );
            }
        })?;
        Ok(())
    }
}

pub fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = terminal::setup()?;
    let mut app = App::new();

    app.add_message("Welcome to Agent Framework TUI");
    app.add_message("Type a message and press Enter to send");
    app.add_message("Press Escape to exit");

    while app.running {
        app.handle_events();
        app.draw(&mut terminal)?;
    }

    terminal::restore()?;
    Ok(())
}
