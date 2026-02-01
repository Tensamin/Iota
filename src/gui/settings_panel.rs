use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    text::Span,
    widgets::{Block, Paragraph},
};

use crate::gui::app_state::AppState;

pub fn draw(frame: &mut Frame, area: Rect, block: Block, password: String, _state: AppState) {
    frame.render_widget(block, area);

    let padded = area.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });

    let info = vec![
        "This is the technical end of your Iota",
        "Press Ctrl+Q to quit",
        "Press Ctrl+R to reload",
        "Select a password for the WebUI",
    ];
    let mut constraints: Vec<Constraint> = Vec::new();

    for _ in 0..info.len() {
        constraints.push(Constraint::Length(1));
    }

    constraints.push(Constraint::Length(3));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(padded);

    for i in 0..info.len() {
        frame.render_widget(Paragraph::new(info[i]), chunks[i]);
    }

    let prefix = Span::raw(format!("select password : {}", password));
    frame.render_widget(Paragraph::new(prefix), chunks[info.len()]);
}
