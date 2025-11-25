use crate::APP_STATE;
use crate::SHUTDOWN;
use crate::gui::ratatui_interface::TERMINAL;
use crate::gui::widgets::betterblock::draw_block_joins;
use crate::langu::language_manager::format;
use crate::langu::language_manager::from_key;

use crate::data::communication::{CommunicationType, CommunicationValue, DataTypes};
use ratatui::widgets::canvas::{Canvas, Line};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Color,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::{thread, time::Duration};
use sysinfo::{RefreshKind, System};

pub fn log_cv(cv: &CommunicationValue) {
    if cv.is_type(CommunicationType::identification_response) {
        let args = [cv.get_data(DataTypes::accepted).unwrap().as_str().unwrap()];
        log_message(format(&"identification_response", &args));
    } else {
        log_message_trans(format!("{:?}", &cv.comm_type));
    }
}
pub fn log_message_trans(key: impl Into<String>) {
    APP_STATE.lock().unwrap().push_log(from_key(&key.into()));
}
pub fn log_message(msg: impl Into<String>) {
    APP_STATE.lock().unwrap().push_log(msg.into());
}
fn smooth_data(data: &[(f64, f64)], window_size: usize) -> Vec<(f64, f64)> {
    if data.len() < window_size {
        return data.to_vec();
    }
    let mut smoothed = Vec::with_capacity(data.len());
    for i in 0..data.len() {
        let start = if i + 1 >= window_size {
            i + 1 - window_size
        } else {
            0
        };
        let window = &data[start..=i];
        let avg = window.iter().map(|(_, y)| y).sum::<f64>() / window.len() as f64;
        smoothed.push((data[i].0, avg));
    }
    smoothed
}

fn downsample_to_fit_width(data: &[(f64, f64)], width: u16) -> Vec<(f64, f64)> {
    let width_usize = (width as usize) * 2;
    let len = data.len();

    if len >= width_usize {
        // Trim data to fit
        data[len - width_usize..].to_vec()
    } else {
        let mut result = Vec::with_capacity(width_usize);

        // Define X spacing (so dummy points are properly spaced across the canvas)
        let dx = 1.0;
        let pad_len = width_usize - len;

        // If we have real data, use its first x position to determine where to start padding
        let start_x = data
            .first()
            .map(|(x, _)| x - (dx * pad_len as f64))
            .unwrap_or(0.0);
        let _ = data.first().map(|(_, y)| *y).unwrap_or(0.0);

        // Fill padding with increasing x positions so they're visible
        for i in 0..pad_len {
            result.push((start_x + i as f64 * dx, -1 as f64));
        }

        // Then append the real data
        result.extend_from_slice(data);
        result
    }
}

pub fn setup() {
    // Start a background thread to sample metrics
    thread::spawn(async move || {
        let mut sys = System::new_with_specifics(RefreshKind::new());
        let mut last_total_received = 0u64;
        let mut last_total_transmitted = 0u64;
        let mut counter = 0.0;
        loop {
            if *SHUTDOWN.read().await {
                break;
            }
            sys.refresh_all();

            let mut tcpu = 0;
            for cpu in sys.cpus() {
                tcpu += cpu.cpu_usage() as i64;
                tcpu /= 2;
            }
            let ram = (sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0;

            let total_received = 0u64;
            let total_transmitted = 0u64;

            let delta_received = if last_total_received == 0 {
                0
            } else {
                total_received.saturating_sub(last_total_received)
            };
            let delta_transmitted = if last_total_transmitted == 0 {
                0
            } else {
                total_transmitted.saturating_sub(last_total_transmitted)
            };
            last_total_received = total_received;
            last_total_transmitted = total_transmitted;

            let net_down = delta_received as f64;
            let net_up = delta_transmitted as f64;

            {
                let mut st = APP_STATE.lock().unwrap();
                st.push_cpu((counter, tcpu as f64));
                st.push_ram((counter, ram));
                st.push_net_down((counter, net_down));
                st.push_net_up((counter, net_up));

                st.sys_info = format!("NetDown: {}  NetUp: {}", delta_received, delta_transmitted);
            }

            counter += 1.0;
            thread::sleep(Duration::from_millis(1000));
        }
    });
}

pub fn render() {
    tokio::spawn(async move {
        TERMINAL
            .lock()
            .await
            .draw(|f| {
                let sys = System::new_all();

                let size = f.area();
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                    .split(size);

                let left = chunks[0];
                let right = chunks[1];

                {
                    let st = APP_STATE.lock().unwrap();
                    let items: Vec<ListItem> = st
                        .logs
                        .iter()
                        .rev()
                        .map(|s| ListItem::new(s.clone()))
                        .collect();
                    let list = List::new(items)
                        .block(Block::default().title("Logs").borders(Borders::ALL));
                    f.render_widget(list, left);
                }

                let right_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(0)])
                    .split(right);

                {
                    let st = APP_STATE.lock().unwrap();
                    let header = Paragraph::new(st.sys_info.clone()).block(
                        Block::default()
                            .title("System Info")
                            .borders(Borders::TOP.union(Borders::LEFT).union(Borders::RIGHT)),
                    );
                    f.render_widget(header, right_chunks[0]);
                }

                let grid_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(49), Constraint::Percentage(51)])
                    .split(right_chunks[1]);

                let left_column = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(49), Constraint::Percentage(51)])
                    .split(grid_chunks[0]);

                let right_column = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(49), Constraint::Percentage(51)])
                    .split(grid_chunks[1]);

                let st = APP_STATE.lock().unwrap();
                let w = grid_chunks[0].width.saturating_sub(2);

                let ping_ds = downsample_to_fit_width(&smooth_data(&st.ping, 3), w + 4);
                let cpu_ds = downsample_to_fit_width(&smooth_data(&st.cpu, 3), w);
                let ram_ds = downsample_to_fit_width(&smooth_data(&st.ram, 3), w);
                let down_ds = downsample_to_fit_width(&st.net_down, w + 4);
                let up_ds = downsample_to_fit_width(&st.net_up, w + 4);

                // ---- Render CPU ----
                {
                    let min_x = cpu_ds.first().map(|(x, _)| *x).unwrap_or(0.0);
                    let max_x = cpu_ds.last().map(|(x, _)| *x).unwrap_or(100.0);
                    let block = Block::default()
                        .title(format!(
                            "CPU {}%",
                            &st.cpu.last().unwrap_or(&(0.0 as f64, 0.0 as f64)).1
                        ))
                        .borders(Borders::TOP.union(Borders::RIGHT).union(Borders::LEFT));
                    let canvas = Canvas::default()
                        .block(block)
                        .x_bounds([min_x, max_x])
                        .y_bounds([0.0, 100.0])
                        .paint(|ctx| {
                            for (x, y) in &cpu_ds {
                                ctx.draw(&Line {
                                    x1: *x,
                                    y1: 0.0,
                                    x2: *x,
                                    y2: *y,
                                    color: Color::Cyan,
                                });
                            }
                        });
                    f.render_widget(canvas, left_column[0]);
                    draw_block_joins(
                        f,
                        left_column[0],
                        Borders::TOP.union(Borders::LEFT),
                        Borders::TOP,
                    );
                    draw_block_joins(
                        f,
                        left_column[0],
                        Borders::TOP.union(Borders::RIGHT),
                        Borders::RIGHT,
                    );
                }

                // ---- Render RAM ----
                {
                    let min_x = ram_ds.first().map(|(x, _)| *x).unwrap_or(0.0);
                    let max_x = ram_ds.last().map(|(x, _)| *x).unwrap_or(100.0);

                    let ram_used = sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
                    let ram_total = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
                    let ram = (sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0;

                    let canvas = Canvas::default()
                        .block(
                            Block::default()
                                .title(format!(
                                    "RAM {:.1}% ({:.1}GiB/{:.1}GiB)",
                                    ram, ram_used, ram_total
                                ))
                                .borders(Borders::ALL),
                        )
                        .x_bounds([min_x, max_x])
                        .y_bounds([0.0, 100.0])
                        .paint(|ctx| {
                            for (x, y) in &ram_ds {
                                ctx.draw(&Line {
                                    x1: *x,
                                    y1: 0.0,
                                    x2: *x,
                                    y2: *y,
                                    color: Color::Magenta,
                                });
                            }
                        });

                    f.render_widget(canvas, left_column[1]);
                    draw_block_joins(
                        f,
                        left_column[1],
                        Borders::ALL,
                        Borders::TOP.union(Borders::RIGHT),
                    );
                }

                // ---- Render Ping ----
                {
                    let min_x = ping_ds.first().map(|(x, _)| *x).unwrap_or(0.0);
                    let max_x = ping_ds.last().map(|(x, _)| *x).unwrap_or(100.0);
                    let max_y = ping_ds.iter().map(|(_, y)| *y).fold(1.0, f64::max);

                    let canvas = Canvas::default()
                        .block(
                            Block::default()
                                .title(format!("Ping {:.1}(ms)", max_y))
                                .borders(Borders::TOP.union(Borders::RIGHT)),
                        )
                        .x_bounds([min_x, max_x])
                        .y_bounds([0.0, max_y])
                        .paint(|ctx| {
                            for (x, y) in &ping_ds {
                                ctx.draw(&Line {
                                    x1: *x,
                                    y1: 0.0,
                                    x2: *x,
                                    y2: *y,
                                    color: Color::Yellow,
                                });
                            }
                        });
                    f.render_widget(canvas, right_column[0]);
                    draw_block_joins(
                        f,
                        right_column[0],
                        Borders::RIGHT.union(Borders::TOP),
                        Borders::TOP,
                    );
                }

                // ---- Render Network ----
                {
                    let min_x = up_ds.first().map(|(x, _)| *x).unwrap_or(0.0);
                    let max_x = up_ds.last().map(|(x, _)| *x).unwrap_or(100.0);
                    let mut max_sum: f64 = 1.0;
                    for ((_, up), (_, down)) in up_ds.iter().zip(down_ds.iter()) {
                        max_sum = max_sum.max(up + down);
                    }

                    let canvas = Canvas::default()
                        .block(
                            Block::default()
                                .title("Network Up/Down")
                                .borders(Borders::TOP.union(Borders::RIGHT).union(Borders::BOTTOM)),
                        )
                        .x_bounds([min_x, max_x])
                        .y_bounds([0.0, max_sum])
                        .paint(|ctx| {
                            for (x, dval) in &down_ds {
                                ctx.draw(&Line {
                                    x1: *x,
                                    y1: 0.0,
                                    x2: *x,
                                    y2: *dval,
                                    color: Color::Red,
                                });
                            }
                            for (x, uval) in &up_ds {
                                let dval = down_ds
                                    .iter()
                                    .find(|(xx, _)| (*xx - *x).abs() < f64::EPSILON)
                                    .map(|(_, y)| *y)
                                    .unwrap_or(0.0);

                                ctx.draw(&Line {
                                    x1: *x,
                                    y1: dval,
                                    x2: *x,
                                    y2: dval + *uval,
                                    color: Color::Green,
                                });
                            }
                        });

                    f.render_widget(canvas, right_column[1]);
                    draw_block_joins(
                        f,
                        right_column[1],
                        Borders::TOP.union(Borders::RIGHT).union(Borders::BOTTOM),
                        Borders::TOP,
                    );
                }
            })
            .unwrap();
    });
}
