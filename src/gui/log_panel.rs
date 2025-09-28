use crate::langu::language_manager::from_key;
use crate::{APP_STATE, omikron::ping_pong_task::PingPongTask};
use crossterm::{
    ExecutableCommand,
    terminal::{LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::widgets::canvas::{Canvas, Line};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::Color,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::{collections::VecDeque, io::stdout, process::Command, thread, time::Duration};
use sysinfo::{RefreshKind, System};

const MAX_POINTS: usize = 1000;
const MAX_LOGS: usize = 100;

#[derive(Clone)]
pub struct AppState {
    logs: VecDeque<String>,
    cpu: Vec<(f64, f64)>,
    ram: Vec<(f64, f64)>,
    ping: Vec<(f64, f64)>,
    net_up: Vec<(f64, f64)>,
    net_down: Vec<(f64, f64)>,
    sys_info: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            logs: VecDeque::new(),
            cpu: Vec::new(),
            ram: Vec::new(),
            ping: Vec::new(),
            net_up: Vec::new(),
            net_down: Vec::new(),
            sys_info: String::from("Loading..."),
        }
    }

    pub fn push_log(&mut self, msg: String) {
        if self.logs.len() >= MAX_LOGS {
            self.logs.pop_front();
        }
        self.logs.push_back(msg);
    }

    pub fn push_cpu(&mut self, pt: (f64, f64)) {
        self.cpu.push(pt);
        if self.cpu.len() > MAX_POINTS {
            self.cpu.remove(0);
        }
    }

    pub fn push_ram(&mut self, pt: (f64, f64)) {
        self.ram.push(pt);
        if self.ram.len() > MAX_POINTS {
            self.ram.remove(0);
        }
    }

    pub fn push_ping_val(&mut self, pt: f64) {
        self.ping.push((self.ping.len() as f64, pt));
        if self.ping.len() > MAX_POINTS {
            self.ping.remove(0);
        }
    }

    pub fn push_net_up(&mut self, pt: (f64, f64)) {
        self.net_up.push(pt);
        if self.net_up.len() > MAX_POINTS {
            self.net_up.remove(0);
        }
    }

    pub fn push_net_down(&mut self, pt: (f64, f64)) {
        self.net_down.push(pt);
        if self.net_down.len() > MAX_POINTS {
            self.net_down.remove(0);
        }
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
    if len == 0 {
        return Vec::new();
    }
    let slice = if len <= width_usize {
        data.to_vec()
    } else {
        data[len - width_usize..].to_vec()
    };
    slice
}

fn measure_ping_ms(host: &str) -> Option<f64> {
    // adapt for Windows
    let output = Command::new("ping")
        .arg("-c")
        .arg("1")
        .arg("-W")
        .arg("1")
        .arg(host)
        .output()
        .ok()?;
    let out = String::from_utf8_lossy(&output.stdout);
    for line in out.lines() {
        if line.contains("time=") {
            if let Some(idx) = line.find("time=") {
                let substr = &line[idx + 5..];
                if let Some(end) = substr.find(" ms") {
                    let num = &substr[..end];
                    if let Ok(f) = num.parse::<f64>() {
                        return Some(f);
                    }
                }
            }
        }
    }
    None
}

pub fn setup() {
    enable_raw_mode().unwrap();

    // Start a background thread to sample metrics
    thread::spawn(move || {
        let mut sys = System::new_with_specifics(RefreshKind::new());
        let mut last_total_received = 0u64;
        let mut last_total_transmitted = 0u64;
        let mut counter = 0.0;
        loop {
            sys.refresh_all();

            let mut tcpu = 0;
            for cpu in sys.cpus() {
                tcpu += cpu.cpu_usage() as i64;
                tcpu /= 2;
            }
            let ram = (sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0;

            let mut total_received = 0u64;
            let mut total_transmitted = 0u64;

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

                st.sys_info = format!(
                    "CPU: {:.1}%  RAM: {:.1}%\nNetDown: {}  NetUp: {}",
                    tcpu, ram, delta_received, delta_transmitted
                );
            }

            counter += 1.0;
            thread::sleep(Duration::from_millis(250));
        }
    });

    // UI rendering loop in a separate thread
    thread::spawn(move || {
        let stdout = stdout();
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout)).unwrap();

        enable_raw_mode().unwrap();

        loop {
            terminal
                .draw(|f| {
                    let size = f.size();
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
                        .constraints([Constraint::Length(5), Constraint::Min(0)])
                        .split(right);

                    {
                        let st = APP_STATE.lock().unwrap();
                        let header = Paragraph::new(st.sys_info.clone())
                            .block(Block::default().title("System Info").borders(Borders::ALL));
                        f.render_widget(header, right_chunks[0]);
                    }

                    let grid_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(right_chunks[1]);

                    let left_column = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(grid_chunks[0]);

                    let right_column = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(grid_chunks[1]);

                    let st = APP_STATE.lock().unwrap();
                    let w = grid_chunks[0].width.saturating_sub(2);

                    let ping_ds = downsample_to_fit_width(&smooth_data(&st.ping, 3), w);
                    let cpu_ds = downsample_to_fit_width(&smooth_data(&st.cpu, 3), w);
                    let ram_ds = downsample_to_fit_width(&smooth_data(&st.ram, 3), w);
                    let down_ds = downsample_to_fit_width(&st.net_down, w);
                    let up_ds = downsample_to_fit_width(&st.net_up, w);

                    // ---- Render CPU ----
                    {
                        let min_x = cpu_ds.first().map(|(x, _)| *x).unwrap_or(0.0);
                        let max_x = cpu_ds.last().map(|(x, _)| *x).unwrap_or(100.0);

                        let canvas = Canvas::default()
                            .block(Block::default().title("CPU %").borders(Borders::ALL))
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
                    }

                    // ---- Render RAM ----
                    {
                        let min_x = ram_ds.first().map(|(x, _)| *x).unwrap_or(0.0);
                        let max_x = ram_ds.last().map(|(x, _)| *x).unwrap_or(100.0);
                        let canvas = Canvas::default()
                            .block(Block::default().title("RAM %").borders(Borders::ALL))
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
                    }

                    // ---- Render Ping ----
                    {
                        let min_x = ping_ds.first().map(|(x, _)| *x).unwrap_or(0.0);
                        let max_x = ping_ds.last().map(|(x, _)| *x).unwrap_or(100.0);
                        let max_y = ping_ds.iter().map(|(_, y)| *y).fold(1.0, f64::max);

                        let canvas = Canvas::default()
                            .block(
                                Block::default()
                                    .title(format!("Ping {}(ms)", max_y))
                                    .borders(Borders::ALL),
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
                                    .borders(Borders::ALL),
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
                    }
                })
                .unwrap();

            thread::sleep(Duration::from_millis(100));
        }
    });
    // Clean up terminal before exit
    disable_raw_mode().unwrap();
    stdout().execute(LeaveAlternateScreen).unwrap();
}
