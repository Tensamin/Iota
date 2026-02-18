use std::{any::Any, sync::Arc};

use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders},
};

use crate::gui::{interaction_result::InteractionResult, screens::screens::Screen, ui::UI};

pub struct MainScreen {
    ui: Arc<UI>,
}

impl MainScreen {
    pub async fn new(ui: Arc<UI>) -> Self {
        MainScreen { ui }
    }
}
impl Screen for MainScreen {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_ui(&self) -> &Arc<UI> {
        &self.ui
    }

    fn render(&self, f: &mut Frame, rect: Rect) {
        f.render_widget(Block::default().title("Main").borders(Borders::ALL), rect);
    }

    fn handle_input(&mut self, event: KeyEvent) -> InteractionResult {
        InteractionResult::Unhandled
    }
}
