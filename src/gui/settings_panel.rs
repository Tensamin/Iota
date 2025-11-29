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

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(padded);

    let prefix = Span::raw(format!("select password : {}", password));

    frame.render_widget(Paragraph::new(prefix), chunks[0]);
}
