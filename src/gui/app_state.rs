use std::collections::VecDeque;

use json::{JsonValue, object};

#[derive(Clone)]
pub struct AppState {
    pub logs: VecDeque<String>,
    pub cpu: Vec<(f64, f64)>,
    pub ram: Vec<(f64, f64)>,
    pub ping: Vec<(f64, f64)>,
    pub net_up: Vec<(f64, f64)>,
    pub net_down: Vec<(f64, f64)>,
    pub sys_info: String,
}
const MAX_POINTS: usize = 1000;
const MAX_LOGS: usize = 100;

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
    pub fn to_json(&self) -> JsonValue {
        let json = object! {
            "cpu" => self.cpu
            .iter()
            .map(|(_, y)| *y)
            .collect::<Vec<f64>>(),
            "ram" => self.ram
            .iter()
            .map(|(_, y)| *y)
            .collect::<Vec<f64>>(),
            "ping" => self
            .ping
            .iter()
            .map(|(_, y)| *y)
            .collect::<Vec<f64>>(),
            "net_up" => self
            .net_up
            .iter()
            .map(|(_, y)| *y)
            .collect::<Vec<f64>>(),
            "net_down" => self
            .net_down
            .iter()
            .map(|(_, y)| *y)
            .collect::<Vec<f64>>(),
        };
        json
    }
}
