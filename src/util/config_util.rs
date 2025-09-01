use json::JsonValue;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

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

    fn load_file(path: &str) -> String {
        if let Ok(mut f) = File::open(path) {
            let mut content = String::new();
            let _ = f.read_to_string(&mut content);
            content
        } else {
            String::new()
        }
    }

    fn save_file(path: &str, content: &str) -> std::io::Result<()> {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(path)?;
        file.write_all(content.as_bytes())
    }

    pub fn load(&mut self) {
        let s = Self::load_file("config.json");
        if !s.is_empty() {
            self.config = json::parse(&s).unwrap_or(JsonValue::new_object());
        }
        if self.config.has_key("ssl_port") {
            if let Some(port) = self.config["ssl_port"].as_i32() {
            }
        }
    }

    pub fn change(&mut self, key: &str, value: JsonValue) {
        self.config[key] = value;
        self.unique = true;
    }

    pub fn update(&mut self) {
        if self.unique {
            let _ = self.save();
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        Self::save_file("config.json", &self.config.dump())
    }
}
