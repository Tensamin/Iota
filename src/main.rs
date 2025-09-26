use base64::Engine;
use base64::engine::general_purpose;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use futures_util::SinkExt;
use json::JsonValue::String;
use json::{self, JsonValue};
use rand::Rng;
use rand_core::OsRng;
use rand_core::RngCore;
use reqwest::header::PUBLIC_KEY_PINS_REPORT_ONLY;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;
use x448::{PublicKey, Secret};

mod data;
mod eula;
mod omikron;
mod users;
mod util;
mod gui {
    pub mod ratatui_interface;
}

mod auth;
mod langu;
use crate::auth::auth_connector::AuthConnector;
use crate::data::communication::{CommunicationType, CommunicationValue, DataTypes};
use crate::eula::*;
use crate::langu::language_creator;
use crate::langu::language_manager;
use crate::omikron::omikron_connection::OmikronConnection;
use crate::omikron::ping_pong_task::PingPongTask;
use crate::users::user_manager::UserManager;
use crate::users::user_profile::UserProfile;
use crate::users::user_profile_full::UserProfileFull;
use crate::util::config_util::{self, CONFIG, ConfigUtil};
use crate::util::file_util;

#[tokio::main]
async fn main() {
    // EULA
    //if !eula_checker::check_eula() {
    //    println!("Please accept the end user license agreement before launching!");
    //    return;
    //}

    // UI
    gui::ratatui_interface::launch(false);

    // LANGUAGE PACK
    language_creator::create_languages();

    // BASIC CONFIGURATION
    CONFIG.lock().unwrap().load();
    if !CONFIG.lock().unwrap().config.has_key("iota_id") {
        CONFIG.lock().unwrap().change("iota_id", Uuid::new_v4());
        CONFIG.lock().unwrap().save();
    }

    // USER MANAGEMENT
    UserManager::load_users().await;

    let mut sb = "".to_string();
    for up in UserManager::get_users() {
        sb = sb + "," + &up.user_id.to_string().as_str();
    }

    if !sb.is_empty() {
        sb.remove(0);
    }
    println!(
        "IOTA ID: {}-####-####-####-############",
        CONFIG
            .lock()
            .unwrap()
            .get_iota_id()
            .to_string()
            .split("-")
            .next()
            .unwrap()
    );
    println!("User ID: {}", sb);

    // IDENTIFICATION ON OMIKRON
    let omikron: OmikronConnection = OmikronConnection::new();
    omikron.connect().await;
    let _ping_pong_task = PingPongTask::new(Arc::new(OmikronConnection::new()));
    omikron
        .send_message(
            CommunicationValue::new(CommunicationType::Identification)
                .add_data(DataTypes::UserIds, String(sb.to_string()))
                .add_data(
                    DataTypes::IotaId,
                    String(CONFIG.lock().unwrap().get_iota_id().to_string()),
                )
                .to_json()
                .to_string()
                .as_mut()
                .to_string(),
        )
        .await;

    loop {}
}
