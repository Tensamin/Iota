use crate::gui::{interaction_result::InteractionResult, screens::screens::Screen};
use crossterm::event::KeyEvent;
use ratatui::{Terminal, backend::CrosstermBackend, init};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

/// UI state and rendering
pub struct UI {
    pub terminal: Arc<Mutex<Terminal<CrosstermBackend<std::io::Stdout>>>>,
    screen: Arc<RwLock<Option<Box<dyn Screen>>>>,
}

impl UI {
    pub fn new() -> Self {
        let terminal = init();
        Self {
            terminal: Arc::new(Mutex::new(terminal)),
            screen: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_screen(&self, screen: Box<dyn Screen>) {
        *self.screen.write().await = Some(screen);
    }

    pub async fn handle_input(self: Arc<Self>, key_event: KeyEvent) {
        let result = {
            let mut guard = self.screen.write().await;
            if let Some(screen) = guard.as_mut() {
                screen.handle_input(key_event)
            } else {
                return;
            }
        };
        match result {
            InteractionResult::OpenScreen { screen } => {
                self.set_screen(screen).await;
            }
            InteractionResult::OpenFutureScreen { screen: fut } => {
                let ui = self.clone();
                let screen = fut.await;
                ui.set_screen(screen).await;
            }
            InteractionResult::CloseScreen => {
                /* TODO
                self.set_screen();
                */
            }
            InteractionResult::Handled => {}
            InteractionResult::Unhandled => {}
        }
    }

    pub async fn render(&self) {
        if let Some(screen) = self.screen.read().await.as_ref() {
            let mut terminal = self.terminal.lock().unwrap();
            terminal
                .draw(|f| {
                    screen.render(f, f.area());
                })
                .unwrap();
        }
    }
}
