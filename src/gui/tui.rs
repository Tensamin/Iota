use std::{
    default,
    io::{Stdout, stdout},
    sync::Arc,
    thread,
    time::Duration,
};

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, enable_raw_mode},
};
use once_cell::sync::Lazy;
use ratatui::{
    Frame, Terminal,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::Color,
    widgets::{
        Block, Borders, List, ListItem,
        canvas::{Canvas, Line},
    },
};
use tokio::{
    self,
    sync::{Mutex, RwLock},
};

use crate::{APP_STATE, SHUTDOWN};

// ****** UTIL ******
fn init_terminal() {
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen).unwrap();
    enable_raw_mode().unwrap();
}

// ****** MAIN ******
pub static UNIQUE: Lazy<RwLock<bool>> = Lazy::new(|| RwLock::new(false));
pub static TERMINAL: Lazy<Arc<Mutex<Terminal<CrosstermBackend<Stdout>>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(
        Terminal::new(CrosstermBackend::new(stdout())).unwrap(),
    ))
});

pub fn start_tui() {
    tokio::spawn(async move {
        init_terminal();
        loop {
            if *SHUTDOWN.read().await {
                break;
            }
            if *UNIQUE.read().await {
                render_tui().await;
            } else {
                thread::sleep(Duration::from_millis(50));
            }
        }
    });
}
pub async fn render_tui() {
    TERMINAL
        .lock()
        .await
        .draw(|f| {
            let area = f.area();
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Percentage(60),
                    Constraint::Max(40),
                ])
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Percentage(60),
                    Constraint::Min(40),
                ])
                .split(area);
            let state;
            {
                state = APP_STATE.lock().unwrap().clone();
            }
            // LOGS
            {
                let items: Vec<ListItem> = state
                    .logs
                    .iter()
                    .rev()
                    .map(|s| ListItem::new(s.clone()))
                    .collect();
                let list = List::new(items).block(
                    Block::default()
                        .title("Logs")
                        .borders(Borders::LEFT.union(Borders::TOP).union(Borders::BOTTOM)),
                );
                f.render_widget(list, chunks[0]);
            }
            // SETTINGS
            {
                let items: Vec<ListItem> = state
                    .logs
                    .iter()
                    .rev()
                    .map(|s| ListItem::new(s.clone()))
                    .collect();
                let list = List::new(items).block(
                    Block::default()
                        .title("Settings")
                        .borders(Borders::LEFT.union(Borders::TOP).union(Borders::BOTTOM)),
                );
                f.render_widget(list, chunks[1]);
            }
            // GRAPHS
            {
                let stack = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Ratio(1, 3),
                            Constraint::Ratio(1, 3),
                            Constraint::Ratio(1, 3),
                        ]
                        .as_ref(),
                    )
                    .split(chunks[2]);
                render_graphs(
                    f,
                    stack[0],
                    "CPU".to_string(),
                    state.with_width(38).cpu,
                    Color::Cyan,
                );
                render_graphs(
                    f,
                    stack[1],
                    "RAM".to_string(),
                    state.with_width(38).ram,
                    Color::Green,
                );
                render_graphs(
                    f,
                    stack[2],
                    "PING".to_string(),
                    state.with_width(38).ping,
                    Color::Magenta,
                );
            }
        })
        .unwrap();
    *UNIQUE.write().await = false;
}

pub fn render_graphs(
    f: &mut Frame<'_>,
    area: Rect,
    title: String,
    graph: Vec<(f64, f64)>,
    color: Color,
) {
    let min_x = graph.first().map(|(x, _)| *x).unwrap_or(0.0);
    let max_x = graph.last().map(|(x, _)| *x).unwrap_or(100.0);
    let min_y = graph
        .iter()
        .map(|(_, y)| *y)
        .filter(|y| *y > 0.0)
        .min_by(|a, b| a.total_cmp(b))
        .unwrap_or(0.0);
    let max_y = graph.iter().map(|(_, y)| *y).fold(f64::MIN, f64::max);
    let block = Block::default()
        .title(format!(
            "{}: {}, {}/{}  MIN/MAX ",
            title,
            graph.last().unwrap_or(&(0.0 as f64, 0.0 as f64)).1 as i64,
            min_y as i64,
            max_y as i64
        ))
        .borders(Borders::ALL);
    let canvas = Canvas::default()
        .block(block)
        .x_bounds([min_x, max_x])
        .y_bounds([0.0, 100.0])
        .paint(|ctx| {
            for (x, y) in &graph {
                ctx.draw(&Line {
                    x1: *x,
                    y1: 0.0,
                    x2: *x,
                    y2: *y,
                    color,
                });
            }
        });
    f.render_widget(canvas, area);
}
