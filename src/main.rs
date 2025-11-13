use json::{self, JsonValue::String};
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use tokio::time::{Duration, sleep};
use uuid::Uuid;

mod auth;
mod communities;
mod data;
mod eula;
mod gui;
mod langu;
mod omikron;
mod server;
mod users;
mod util;

use crate::communities::community_manager;
use crate::communities::interactables::registry;
use crate::data::communication::{CommunicationType, CommunicationValue, DataTypes};
use crate::gui::app_state::AppState;
use crate::gui::log_panel::{log_message, log_message_trans};
use crate::gui::{log_panel, ratatui_interface};
use crate::langu::language_creator;
use crate::langu::language_manager::format;
use crate::omikron::omikron_connection::OmikronConnection;
use crate::server::server::start;
use crate::users::user_manager;
use crate::util::config_util::CONFIG;

pub static APP_STATE: LazyLock<Arc<Mutex<AppState>>> =
    LazyLock::new(|| Arc::new(Mutex::new(AppState::new())));
#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
#[allow(unused_must_use, dead_code)]
async fn main() {
    // EULA
    //if !eula_checker::check_eula() {
    //    println!("Please accept the end user license agreement before launching!");
    //    return;
    //}

    // LANGUAGE PACK
    if let Err(e) = language_creator::create_languages() {
        println!("Language pack creation failed: {}", e);
        return;
    }

    // UI
    log_panel::setup();
    if let Err(e) = ratatui_interface::launch() {
        println!("Ui launch failed: {}", &e.to_string());
        return;
    }

    // BASIC CONFIGURATION
    CONFIG.lock().unwrap().load();
    if !CONFIG.lock().unwrap().config.has_key("iota_id") {
        CONFIG.lock().unwrap().change("iota_id", Uuid::new_v4());
        CONFIG.lock().unwrap().update();
    }

    // USER MANAGEMENT
    if let Err(_) = user_manager::load_users().await {
        log_message_trans("user_load_failed");
    }

    let mut sb = "".to_string();

    for up in user_manager::get_users() {
        sb = sb + "," + &up.user_id.to_string().as_str();
    }

    if !sb.is_empty() {
        {}
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

    // COMMUNITY MANAGEMENT
    registry::load_interactables().await;
    community_manager::load_communities().await;
    community_manager::save_communities().await;
    let mut sb1 = "".to_string();
    for cp in community_manager::get_communities().await {
        sb1 = sb1 + "," + &cp.get_name().to_string().as_str();
    }

    if !sb1.is_empty() {
        sb1.remove(0);
        sb1 = sb1 + ",";
    }
    log_message(format!("Community IDS: {}", sb1));
    let port = CONFIG.lock().unwrap().get_port();
    if start(port).await {
        log_message(format("community_active", &[&port.to_string()]));
    } else {
        if port < 1024 {
            log_message(format("community_start_error_admin", &[&port.to_string()]));
        } else {
            log_message(format("community_start_error", &[&port.to_string()]));
        }
    }

    loop {
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
        log_message_trans("setup_completed");
        loop {
            if !omikron.is_connected().await {
                break;
            }
            sleep(Duration::from_secs(1)).await;
        }
    }
}
