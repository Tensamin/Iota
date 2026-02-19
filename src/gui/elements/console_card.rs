use actix_web::web::block;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::gui::{
    elements::elements::{Element, InteractableElement, JoinableElement},
    interaction_result::InteractionResult,
    util::borders::draw_block_joins,
};
use std::any::Any;

pub struct ConsoleCard {
    focused: bool,
    pub title: String,
    pub content: String,

    borders: Borders,
    joins: Borders,
}

impl ConsoleCard {
    pub fn new(title: &str, content: &str) -> Self {
        ConsoleCard {
            focused: false,
            title: title.to_string(),
            content: content.to_string(),
            borders: Borders::ALL,
            joins: Borders::NONE,
        }
    }
}

impl Element for ConsoleCard {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn render(&self, f: &mut Frame, r: Rect) {
        let block = Block::default()
            .borders(self.borders)
            .title(self.title.clone())
            .title_style(Style::default().fg(Color::White))
            .border_style(if self.focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .style(if self.focused {
                Style::default().fg(Color::White)
            } else {
                Style::default()
            });

        let par = Paragraph::new(Line::from(Span::from(self.content.clone())))
            .block(block)
            .scroll((0, 0));
        f.render_widget(par, r);
        draw_block_joins(f, r, self.borders, self.joins);
    }
}

impl JoinableElement for ConsoleCard {
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

impl InteractableElement for ConsoleCard {
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
            KeyCode::Enter => {
                self.content = "".to_string();
                InteractionResult::Handled
            }
            KeyCode::Backspace => {
                self.content.pop();
                InteractionResult::Handled
            }
            _ => {
                if let Some(c) = key.code.as_char() {
                    self.content.push(c);
                    InteractionResult::Handled
                } else {
                    InteractionResult::Unhandled
                }
            }
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
