use std::{io::Stdout, sync::Arc, time::Duration};

use crate::{
    APP_STATE, SHUTDOWN,
    gui::{settings_panel, widgets::betterblock::draw_block_joins},
    util::config_util::CONFIG,
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

pub static UNIQUE: Lazy<RwLock<bool>> = Lazy::new(|| RwLock::new(true));
pub static TERMINAL: Lazy<Arc<Mutex<Terminal<CrosstermBackend<Stdout>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(ratatui::init())));

pub fn start_tui() {
    tokio::spawn(async move {
        loop {
            if *SHUTDOWN.read().await {
                break;
            }

            if *UNIQUE.read().await {
                render_tui().await;
            } else {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
        ratatui::restore();
    });
}

pub async fn render_tui() {
    let password = CONFIG
        .read()
        .await
        .get("password")
        .as_str()
        .unwrap_or("")
        .to_string();

    let mut terminal = TERMINAL.lock().await;
    let _ = terminal.draw(|f| {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Percentage(60),
                Constraint::Min(40),
            ])
            .split(area);

        let state = APP_STATE.lock().unwrap().clone();

        // LOGS PANEL
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
        // SETTINGS PANEL
        {
            settings_panel::draw(
                f,
                chunks[1],
                Block::default()
                    .title("Settings")
                    .borders(Borders::LEFT.union(Borders::TOP).union(Borders::BOTTOM)),
                password,
                state.clone(),
            );
            draw_block_joins(
                f,
                chunks[1],
                Borders::TOP.union(Borders::LEFT).union(Borders::BOTTOM),
                Borders::LEFT,
            );
        }
        // PERFORMANCE GRAPHS
        {
            let stack = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                ])
                .split(chunks[2]);

            render_graphs(
                f,
                stack[0],
                "CPU".into(),
                "%".into(),
                state.with_width(38).cpu,
                Borders::TOP.union(Borders::LEFT).union(Borders::RIGHT),
                Color::Cyan,
            );
            draw_block_joins(
                f,
                stack[0],
                Borders::TOP.union(Borders::LEFT),
                Borders::LEFT,
            );

            render_graphs(
                f,
                stack[1],
                "RAM".into(),
                "%".into(),
                state.with_width(38).ram,
                Borders::TOP.union(Borders::LEFT).union(Borders::RIGHT),
                Color::Green,
            );
            draw_block_joins(
                f,
                stack[1],
                Borders::TOP.union(Borders::LEFT).union(Borders::RIGHT),
                Borders::TOP,
            );

            render_graphs(
                f,
                stack[2],
                "PING".into(),
                "ms".into(),
                state.with_width(38).ping,
                Borders::ALL,
                Color::Magenta,
            );
            draw_block_joins(f, stack[2], Borders::ALL, Borders::TOP);
            draw_block_joins(
                f,
                stack[2],
                Borders::BOTTOM.union(Borders::LEFT),
                Borders::LEFT,
            );
        }
    });

    *UNIQUE.write().await = false;
}

pub fn render_graphs(
    f: &mut Frame<'_>,
    area: Rect,
    title: String,
    unit: String,
    graph: Vec<(f64, f64)>,
    borders: Borders,
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
            "─{}:─{}{}──{}/{}─MIN/MAX",
            title,
            graph.last().unwrap_or(&(0.0, 0.0)).1 as i64,
            unit,
            min_y as i64,
            max_y as i64
        ))
        .borders(borders);

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
