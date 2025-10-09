use json::JsonValue::String;
use json::{self};
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use uuid::Uuid;

mod auth;
mod data;
mod eula;
mod gui;
mod langu;
mod omikron;
mod users;
mod util;

use crate::data::communication::{CommunicationType, CommunicationValue, DataTypes};
use crate::gui::app_state::AppState;
use crate::gui::log_panel::{log_message, log_message_trans};
use crate::gui::{log_panel, ratatui_interface};
use crate::langu::language_creator;
use crate::omikron::omikron_connection::OmikronConnection;
use crate::users::user_manager::UserManager;
use crate::util::config_util::CONFIG;

pub static APP_STATE: LazyLock<Arc<Mutex<AppState>>> =
    LazyLock::new(|| Arc::new(Mutex::new(AppState::new())));

#[tokio::main]
async fn main() {
    // EULA
    //if !eula_checker::check_eula() {
    //    println!("Please accept the end user license agreement before launching!");
    //    return;
    //}

    // UI
    log_panel::setup();
    ratatui_interface::launch();
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
        sb = sb + ",";
    }
    log_message(format!(
        "IOTA ID:  {}-####-####-####-############",
        CONFIG
            .lock()
            .unwrap()
            .get_iota_id()
            .to_string()
            .split("-")
            .next()
            .unwrap()
    ));
    log_message(format!("User IDS: {}", sb));

    // IDENTIFICATION ON OMIKRON
    let omikron: OmikronConnection = OmikronConnection::new();
    omikron.connect().await;
    omikron
        .send_message(
            CommunicationValue::new(CommunicationType::identification)
                .add_data(DataTypes::user_ids, String(sb.to_string()))
                .add_data(
                    DataTypes::iota_id,
                    String(CONFIG.lock().unwrap().get_iota_id().to_string()),
                )
                .to_json()
                .to_string()
                .as_mut()
                .to_string(),
        )
        .await;
    log_message_trans("SETUP_COMPLETED");

    loop {}
}
