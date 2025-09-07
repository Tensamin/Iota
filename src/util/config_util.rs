use json::JsonValue;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use uuid::Uuid;
use crate::util::file_util::{load_file, save_file};

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
        save_file("","config.json", &self.config.to_string());
    }
}
