use std::fs;
use std::sync::Mutex;
use std::io;
use std::path::Path;
use uuid::Uuid;
use rand::Rng;
use rand::rngs::OsRng;
use base64::{engine::general_purpose, Engine as _};
use json::{JsonValue};
use once_cell::sync::Lazy;
use crate::users::user_profile::UserProfile;
use crate::users::user_profile_full::UserProfileFull;
use crate::util::file_util::{load_file, save_file};

pub struct UserManager;

static USERS: Lazy<Mutex<Vec<UserProfile>>> = Lazy::new(|| Mutex::new(Vec::new()));
static UNIQUE: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

impl UserManager {
    pub fn create_user(username: &str) -> Option<UserProfileFull> {
        // Stub: normally AuthConnector.getRegister() returns a UUID
        let user_id = Uuid::new_v4();

        // Stubbed: CryptoHelper.generateKeyPair()
        let public_key = general_purpose::STANDARD.encode(b"dummy-public");
        let private_key = general_purpose::STANDARD.encode(b"dummy-private");
        let private_key_hash = format!("hash-{}", &private_key);

        let mut bytes = [0u8; 192];
        OsRng.fill(bytes.as_mut());
        let reset_token = general_purpose::STANDARD.encode(&bytes);

        let up = UserProfile::new(
            user_id,
            username.to_string(),
            None,
            public_key,
            private_key_hash,
            reset_token,
        );

        let up_full = UserProfileFull { user_profile: up.clone(), private_key };

        USERS.lock().unwrap().push(up);
        Self::save_users().ok();
        Some(up_full)
    }

    pub fn get_user(user_id: Uuid) -> Option<UserProfile> {
        USERS.lock().unwrap().iter().cloned().find(|u| u.user_id == user_id)
    }

    pub fn get_users() -> Vec<UserProfile> {
        USERS.lock().unwrap().clone()
    }

    pub fn add_user(up: UserProfile) {
        let mut users = USERS.lock().unwrap();
        users.retain(|u| u.user_id != up.user_id);
        users.push(up);
        *UNIQUE.lock().unwrap() = true;
    }

    pub fn remove_user(user_id: Uuid) {
        let mut users = USERS.lock().unwrap();
        users.retain(|u| u.user_id != user_id);
        *UNIQUE.lock().unwrap() = true;
    }

    pub fn save_users() -> io::Result<()> {
        *UNIQUE.lock().unwrap() = false;
        let users = USERS.lock().unwrap();
        let arr: Vec<JsonValue> = users.iter().map(|u| u.to_json()).collect();
        let json_str = JsonValue::Array(arr).dump();

        save_file("", "users.json", &json_str);
        Ok(())
    }

    pub async fn load_users() -> io::Result<()> {
        let content = load_file("", "users.json");
        if content.trim().is_empty() {
            return Ok(());
        }

        let parsed = json::parse(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        if let JsonValue::Array(arr) = parsed {
            let mut users = USERS.lock().unwrap();
            for j in arr.iter() {
                if let Some(up) = UserProfile::from_json(j).await {
                    users.push(up);
                }
            }
        }
        if *UNIQUE.lock().unwrap() {
            Self::save_users().ok();
        }
        Ok(())
    }

    pub fn set_unique(val: bool) {
        *UNIQUE.lock().unwrap() = val;
    }
}
