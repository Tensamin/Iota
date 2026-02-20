use crate::gui::elements::elements::{InteractableElement, JoinableElement};
use crate::gui::interaction_result::InteractionResult;
use crate::gui::util::borders::draw_block_joins;
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

#[derive(Clone)]
pub struct UiLogEntry {
    pub line: String,
    pub color: Color,
}

pub struct LogCard {
    focused: bool,
    scroll: u16,

    pub borders: Borders,
    pub joins: Borders,
}

impl LogCard {
    pub fn new() -> Self {
        Self {
            focused: false,
            scroll: 1,
            borders: Borders::ALL,
            joins: Borders::NONE,
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
            .map(|log| {
                Line::from(Span::raw(log.line.clone())).style(Style::default().fg(log.color))
            })
            .collect();

        let block = Block::default()
            .title(if self.focused {
                "Logs J/K to scroll"
            } else {
                "Logs"
            })
            .borders(self.borders)
            .border_style(if self.focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            });

        let inner_height = area.height.saturating_sub(2) as usize;

        let total_lines = lines.len();
        let base_scroll = total_lines.saturating_sub(inner_height) as u16;

        let scroll = base_scroll.saturating_sub(self.scroll);

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));

        f.render_widget(paragraph, area);
        draw_block_joins(f, area, self.borders, self.joins);
    }
}
impl JoinableElement for LogCard {
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

    fn set_borders(&mut self, borders: Borders) {
        self.borders = borders;
    }

    fn set_joins(&mut self, joins: Borders) {
        self.joins = joins;
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
            KeyCode::Char('J') | KeyCode::Char('j') => {
                let state = APP_STATE.lock().unwrap();
                let logs = state.get_logs();
                if self.scroll < logs.len() as u16 {
                    self.scroll += 1;
                }
                InteractionResult::Handled
            }
            KeyCode::Char('K') | KeyCode::Char('k') => {
                if self.scroll > 1 {
                    self.scroll -= 1;
                }
                InteractionResult::Handled
            }
            _ => InteractionResult::Unhandled,
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
