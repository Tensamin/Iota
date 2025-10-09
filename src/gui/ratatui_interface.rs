use std::{
    io::Stdout,
    sync::{Arc, LazyLock},
    time::Duration,
};

use crate::gui::nav_bar::NavBar;
use color_eyre::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, enable_raw_mode},
};
use futures_util::lock::Mutex;
use ratatui::{Terminal, prelude::CrosstermBackend};
use std::io::stdout;
use tokio::task;

pub static TERMINAL: LazyLock<Arc<Mutex<Terminal<CrosstermBackend<Stdout>>>>> =
    LazyLock::new(|| {
        Arc::new(Mutex::new(
            Terminal::new(CrosstermBackend::new(stdout())).unwrap(),
        ))
    });
pub static NAV_BAR: LazyLock<Arc<Mutex<NavBar>>> =
    LazyLock::new(|| Arc::new(Mutex::new(NavBar::new())));

pub fn launch() -> Result<()> {
    color_eyre::install()?;
    task::spawn(run());
    ratatui::restore();
    Ok(())
}

fn init_terminal() {
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen).unwrap();
    enable_raw_mode().unwrap();
}
async fn run() -> Result<()> {
    init_terminal();
    loop {
        NAV_BAR.lock().await.current_screen.renderf();
        tokio::time::sleep(Duration::from_millis(1000)).await;
    }
}
