use crate::auth::auth_connector;
use crate::gui::log_panel::log_message;
use crate::users::user_manager::UserManager;
use base64::{Engine as _, engine::general_purpose};
use json::{JsonValue, object, stringify};
use rand::Rng;
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Mutex;
use uuid::Uuid;

// --- UserProfile ---
#[derive(Clone, Debug)]
pub struct UserProfile {
    pub user_id: Uuid,
    pub username: String,
    pub public_key: String,
    pub private_key_hash: String,
    pub reset_token: String,
    pub display_name: Option<String>,
}

impl UserProfile {
    pub fn new(
        user_id: Uuid,
        username: String,
        display_name: Option<String>,
        public_key: String,
        private_key_hash: String,
        reset_token: String,
    ) -> Self {
        Self {
            user_id,
            username,
            display_name,
            public_key,
            private_key_hash,
            reset_token,
        }
    }

    pub fn to_json(&self) -> JsonValue {
        let mut obj = object! {
            "uuid" => self.user_id.to_string(),
            "username" => self.username.clone(),
            "public_key" => self.public_key.clone(),
            "private_key_hash" => self.private_key_hash.clone(),
            "reset_token" => self.reset_token.clone()
        };
        if let Some(d) = &self.display_name {
            obj["display_name"] = d.clone().into();
        }
        obj
    }

    pub async fn from_json(j: &JsonValue) -> Option<Self> {
        let uuid = Uuid::parse_str(j["uuid"].as_str()?).ok()?;
        let username = j["username"].as_str()?.to_string();
        let public_key = j["public_key"].as_str()?.to_string();
        let private_key_hash = j["private_key_hash"].as_str()?.to_string();
        let reset_token = j["reset_token"].as_str()?.to_string();
        let display_name = j["display_name"].as_str().map(|s| s.to_string());

        let mut up = UserProfile::new(
            uuid,
            username,
            display_name,
            public_key,
            private_key_hash,
            reset_token,
        );

        if j.has_key("migrate")
            || j.has_key("migrating")
            || j.has_key("changing")
            || j.has_key("move")
            || j.has_key("moving")
        {
            if auth_connector::migrate_user(&mut up).await {
                log_message(format!("[INFO] Migration triggered for {}", up.username));
                UserManager::set_unique(true);
            }
        }

        Some(up)
    }

    pub fn randomize_reset_token(&mut self) -> String {
        let mut bytes = [0u8; 192];
        OsRng.fill(bytes.as_mut());
        let new_token = general_purpose::STANDARD.encode(&bytes);
        self.reset_token = new_token.clone();
        new_token
    }

    pub fn get_display_name(&self) -> String {
        self.display_name
            .clone()
            .unwrap_or_else(|| self.username.clone())
    }
}
