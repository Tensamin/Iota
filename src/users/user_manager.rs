use crate::auth::auth_connector;
use crate::users::user_profile::UserProfile;
use crate::util::config_util::CONFIG;
use crate::util::file_util::{load_file, save_file};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use hex::{self};
use json::JsonValue;
use once_cell::sync::Lazy;
use rand::Rng;
use rand_core::OsRng;
use rand_core::RngCore;
use sha2::{Digest, Sha256};
use std::io;
use std::sync::Mutex;
use uuid::Uuid;
use x448::{PublicKey, Secret};

static USERS: Lazy<Mutex<Vec<UserProfile>>> = Lazy::new(|| Mutex::new(Vec::new()));
static UNIQUE: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

pub async fn create_user(username: &str) -> (Option<UserProfile>, Option<String>) {
    let user_id = auth_connector::get_register().await.unwrap();
    let mut buf = [0u8; 56];
    let mut rng = OsRng;
    rng.fill_bytes(&mut buf);
    let private_key = Secret::from_bytes(&buf).unwrap();
    let public_key = PublicKey::from(&private_key);

    let mut hasher = Sha256::new();
    hasher.update(&STANDARD.encode(&private_key.as_bytes()).as_bytes());
    let result = hasher.finalize();
    let private_key_hash = hex::encode(result);

    let mut bytes = [0u8; 192];
    OsRng.fill(bytes.as_mut());
    let reset_token = STANDARD.encode(&bytes);

    let up = UserProfile::new(
        user_id,
        username.to_string(),
        None,
        STANDARD.encode(&public_key.as_bytes()),
        private_key_hash,
        reset_token,
    );

    auth_connector::complete_register(&up, &CONFIG.lock().unwrap().get_iota_id().to_string()).await;
    save_file(
        "",
        &format!("{}.tu", username),
        &format!("{}::{}", user_id, STANDARD.encode(&private_key.as_bytes())),
    );

    USERS.lock().unwrap().push(up.clone());
    save_users().ok();
    (Some(up), Some(STANDARD.encode(&private_key.as_bytes())))
}

pub fn get_user(user_id: Uuid) -> Option<UserProfile> {
    USERS
        .lock()
        .unwrap()
        .iter()
        .cloned()
        .find(|u| u.user_id == user_id)
}

pub fn get_users() -> Vec<UserProfile> {
    USERS.lock().unwrap().clone()
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

    let parsed =
        json::parse(&content).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    if let JsonValue::Array(arr) = parsed {
        let mut users = USERS.lock().unwrap();
        for j in arr.iter() {
            if let Some(up) = UserProfile::from_json(j).await {
                users.push(up);
            }
        }
    }
    if *UNIQUE.lock().unwrap() {
        save_users().ok();
    }
    Ok(())
}

pub fn set_unique(val: bool) {
    *UNIQUE.lock().unwrap() = val;
}
