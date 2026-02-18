use crate::gui::elements::elements::InteractableElement;
use crate::gui::interaction_result::InteractionResult;
use crate::{APP_STATE, gui::elements::elements::Element};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::any::Any;
use std::sync::Arc;

#[derive(Clone)]
pub struct UiLogEntry {
    pub line: String,
    pub color: Color,
}

pub struct LogCard {
    focused: bool,
    scroll: u16,
}

impl LogCard {
    pub fn new() -> Self {
        Self {
            focused: false,
            scroll: 0,
        }
    }
}
impl Element for LogCard {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn render(&self, f: &mut Frame, area: Rect) {
        let state = APP_STATE.lock().unwrap();
        let logs = state.get_logs();

        let lines: Vec<Line> = logs
            .iter()
            .map(|log| Line::from(Span::raw(log.line.clone())).style(log.color))
            .collect();

        let block = Block::default()
            .title("Logs")
            .borders(Borders::ALL)
            .border_style(if self.focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            });

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));

        f.render_widget(paragraph, area);
    }
}
impl InteractableElement for LogCard {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_element(&self) -> &dyn Element {
        self
    }

    fn as_element_mut(&mut self) -> &mut dyn Element {
        self
    }

    fn interact(&mut self, key: KeyEvent) -> InteractionResult {
        match key.code {
            KeyCode::Up => {
                if self.scroll > 0 {
                    self.scroll -= 1;
                }
                InteractionResult::Handeled
            }
            KeyCode::Down => {
                self.scroll += 1;
                InteractionResult::Handeled
            }
            _ => InteractionResult::Unhandeled,
        }
    }

    fn can_focus(&self) -> bool {
        true
    }

    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self, f: bool) {
        self.focused = f;
    }
}
