use crate::{
    ACTIVE_TASKS, SHUTDOWN,
    gui::{
        input_handler::setup_input_handler, interaction_result::InteractionResult,
        screens::screens::Screen,
    },
};
use crossterm::event::KeyEvent;
use once_cell::sync::Lazy;
use ratatui::{Terminal, backend::CrosstermBackend, init};
use std::{
    io::Stdout,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{sync::RwLock, time::Instant};

/// UI state and rendering
pub static UNIQUE: Lazy<RwLock<bool>> = Lazy::new(|| RwLock::new(true));

pub static FPS: Lazy<RwLock<f64>> = Lazy::new(|| RwLock::new(0.0));
pub struct UI {
    pub terminal: Arc<Mutex<Terminal<CrosstermBackend<Stdout>>>>,
    screen: Arc<RwLock<Option<Box<dyn Screen>>>>,
}

pub fn start_tui() -> Arc<UI> {
    let ui = Arc::new(UI::new());
    let uic = ui.clone();
    tokio::spawn(async move {
        ACTIVE_TASKS.insert("UI Renderer".to_string());
        let mut last_render = Instant::now();
        let mut last: Vec<f64> = Vec::new();
        loop {
            if *SHUTDOWN.read().await {
                break;
            }

            if *UNIQUE.read().await {
                uic.render().await;
                let elapsed = last_render.elapsed().as_secs_f64();
                if elapsed > 0.0 {
                    last.push(1.0 / elapsed);
                }
                if last.len() > 10 {
                    last.remove(0);
                }
                *FPS.write().await = last.iter().sum::<f64>() / last.len() as f64;
                last_render = Instant::now();
            }
            tokio::time::sleep(Duration::from_millis(16)).await;
        }
        ACTIVE_TASKS.remove("UI Renderer");
        ratatui::restore();
    });
    setup_input_handler(ui.clone());
    ui
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
                *self.screen.write().await = None;
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
