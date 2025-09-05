use std::fs;
use std::sync::Mutex;
use std::collections::HashMap;
use std::io;
use std::path::Path;
use uuid::Uuid;
use rand::Rng;
use rand::rngs::OsRng;
use base64::{engine::general_purpose, Engine as _};
use json::{JsonValue, object, stringify};
use crate::users::user_manager::UserManager;
use crate::auth::auth_connector::AuthConnector;

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
            "UUID" => self.user_id.to_string(),
            "username" => self.username.clone(),
            "publicKey" => self.public_key.clone(),
            "privateKeyHash" => self.private_key_hash.clone(),
            "resetToken" => self.reset_token.clone()
        };
        if let Some(d) = &self.display_name {
            obj["displayName"] = d.clone().into();
        }
        obj
    }

    pub async fn from_json(j: &JsonValue) -> Option<Self> {
        let uuid = Uuid::parse_str(j["UUID"].as_str()?).ok()?;
        let username = j["username"].as_str()?.to_string();
        let public_key = j["publicKey"].as_str()?.to_string();
        let private_key_hash = j["privateKeyHash"].as_str()?.to_string();
        let reset_token = j["resetToken"].as_str()?.to_string();
        let display_name = j["displayName"].as_str().map(|s| s.to_string());

        let mut up = UserProfile::new(uuid, username, display_name, public_key, private_key_hash, reset_token);

        // Migration hook (stubbed, since AuthConnector isn’t implemented here)
        if j.has_key("migrate") 
        || j.has_key("migrating") 
        || j.has_key("changing") 
        || j.has_key("move") 
        || j.has_key("moving") {
            if AuthConnector::migrate_user(&mut up, stringify!("{}", Uuid::new_v4())).await {
                println!("[INFO] Migration triggered for {}", up.username);
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
        UserManager::save_users().ok();
        new_token
    }

    pub fn get_display_name(&self) -> String {
        self.display_name.clone().unwrap_or_else(|| self.username.clone())
    }
}