use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_chat_area(
    frame: &mut Frame,
    messages: &[String],
    current_input: &str,
    cursor_visible: bool,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let messages_text: Vec<Line> = messages.iter().map(|m| Line::from(m.as_str())).collect();
    let messages_widget = Paragraph::new(messages_text)
        .block(Block::default().borders(Borders::ALL).title("Chat"));
    frame.render_widget(messages_widget, chunks[0]);

    let input_style = if cursor_visible {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let input_widget = Paragraph::new(format!("> {}", current_input))
        .style(input_style)
        .block(Block::default().borders(Borders::ALL).title("Prompt"));
    frame.render_widget(input_widget, chunks[1]);
}

pub fn render_thinking(frame: &mut Frame, text: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(frame.area());
    
    let thinking_widget = Paragraph::new(text)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL).title("Agent"));
    frame.render_widget(thinking_widget, chunks[0]);
}
