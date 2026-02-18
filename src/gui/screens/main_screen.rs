use crate::gui::{
    elements::{
        elements::{Element, InteractableElement},
        log_card::LogCard,
    },
    interaction_result::InteractionResult,
    screens::screens::Screen,
    ui::UI,
};
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    widgets::{Block, Borders},
};
use std::{any::Any, sync::Arc};

pub struct MainScreen {
    ui: Arc<UI>,
    log_card: LogCard,
}

impl MainScreen {
    pub async fn new(ui: Arc<UI>) -> Self {
        let mut log_card = LogCard::new();
        log_card.focus(true);
        MainScreen { ui, log_card }
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
        let main_block = Block::default().title("Main").borders(Borders::ALL);

        f.render_widget(main_block, rect);

        let inner = rect.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        let chunks = Layout::vertical([Constraint::Min(0)]).split(inner);

        self.log_card.render(f, chunks[0]);
    }

    fn handle_input(&mut self, event: KeyEvent) -> InteractionResult {
        if self.log_card.is_focused() {
            return self.log_card.interact(event);
        }

        InteractionResult::Unhandled
    }
}
