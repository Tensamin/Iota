use crate::util::file_util::{self, load_file};
use json::{JsonValue, parse};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone)]
pub struct LanguagePack {
    languages: HashMap<String, String>,
}

// Language packs need to have formatting
// Variables need to be provided

pub static LANGUAGE_PACK: Lazy<Mutex<LanguagePack>> =
    Lazy::new(|| Mutex::new(LanguagePack::new("en_INT")));

pub fn get_language() -> LanguagePack {
    LANGUAGE_PACK.lock().unwrap().clone()
}

impl LanguagePack {
    pub fn new(language: &str) -> Self {
        let mut pack = LanguagePack {
            languages: HashMap::new(),
        };
        pack.load_language(language);
        pack
    }

    pub fn load_language(&mut self, language: &str) {
        let path = format!("languages/{}/", language);

        let frontend_messages = file_util::load_file(&path, "frontend.json");
        let frontend_messages = parse(&frontend_messages).unwrap();
        for (key, value) in frontend_messages.entries() {
            self.languages
                .insert(key.to_string(), value.as_str().unwrap().to_string());
        }

        let omikron_messages = file_util::load_file(&path, "omikron.json");
        let omikron_messages = parse(&omikron_messages).unwrap();
        for (key, value) in omikron_messages.entries() {
            self.languages
                .insert(key.to_string(), value.as_str().unwrap().to_string());
        }

        let button_texts = file_util::load_file(&path, "buttons.json");
        let button_texts = parse(&button_texts).unwrap();
        for (key, value) in button_texts.entries() {
            self.languages
                .insert(key.to_string(), value.as_str().unwrap().to_string());
        }

        let debug_messages = file_util::load_file(&path, "debug.json");
        let debug_messages = parse(&debug_messages).unwrap();
        for (key, value) in debug_messages.entries() {
            self.languages
                .insert(key.to_string(), value.as_str().unwrap().to_string());
        }
    }

    pub fn get_translation(&self, key: &str) -> &String {
        self.languages.get(key).unwrap()
    }
}
