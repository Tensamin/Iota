use crate::util::file_util::{load_file, save_file};
use json::JsonValue;
use once_cell::sync::Lazy;
use std::fs::{self, File};
use std::path::Path;
use std::sync::Mutex;
use uuid::Uuid;

pub static CONFIG: Lazy<Mutex<ConfigUtil>> = Lazy::new(|| Mutex::new(ConfigUtil::new()));

pub struct ConfigUtil {
    pub config: JsonValue,
    pub unique: bool,
}

impl ConfigUtil {
    pub fn new() -> Self {
        Self {
            config: JsonValue::new_object(),
            unique: false,
        }
    }

    pub fn load(&mut self) {
        let s = load_file("", "config.json");
        if !s.is_empty() {
            self.config = json::parse(&s).unwrap_or(JsonValue::new_object());
        }
    }

    pub fn get_iota_id(&self) -> Uuid {
        self.config["iota_id"]
            .as_str()
            .unwrap_or_default()
            .parse()
            .unwrap_or_default()
    }

    pub fn change(&mut self, key: &str, value: Uuid) {
        self.config[key] = JsonValue::String(value.to_string());
        self.unique = true;
    }

    pub fn update(&mut self) {
        if self.unique {
            let _ = self.save();
        }
    }

    pub fn save(&self) {
        save_file("", "config.json", &self.config.to_string());
    }
}
