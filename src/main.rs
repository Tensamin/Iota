use json::JsonValue;
use json::number::Number;
use json::{self};
use once_cell::sync::Lazy;
use pnet::datalink::NetworkInterface;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep};

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
use crate::gui::input_handler;
use crate::gui::log_panel;
use crate::gui::log_panel::{log_message, log_message_trans};
use crate::gui::tui;
use crate::langu::language_creator;
use crate::langu::language_manager::format;
use crate::omikron::omikron_connection::OMIKRON_CONNECTION;
use crate::omikron::omikron_connection::OmikronConnection;
use crate::server::server::start;
use crate::users::user_manager;
use crate::util::config_util::CONFIG;
use crate::util::file_util::has_dir;

pub static APP_STATE: LazyLock<Arc<Mutex<AppState>>> =
    LazyLock::new(|| Arc::new(Mutex::new(AppState::new())));

pub static SHUTDOWN: Lazy<RwLock<bool>> = Lazy::new(|| RwLock::new(false));
pub static RELOAD: Lazy<RwLock<bool>> = Lazy::new(|| RwLock::new(true));
pub static ACTIVE_TASKS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
#[allow(unused_must_use, dead_code)]
async fn main() {
    while *RELOAD.read().await {
        *RELOAD.write().await = false;
        *SHUTDOWN.write().await = false;

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
        tui::start_tui();
        input_handler::setup_input_handler();

        // BASIC CONFIGURATION
        &CONFIG.write().await.load();
        if !CONFIG.read().await.config.has_key("iota_id") {
            CONFIG.write().await.change(
                "iota_id",
                JsonValue::Number(Number::from(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or(Duration::from_millis(0))
                        .as_millis() as i64,
                )),
            );
            CONFIG.write().await.update();
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
            "IOTA ID:  {}",
            CONFIG.read().await.get_iota_id().to_string()
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
        let port = CONFIG.read().await.get_port();
        let mut ip = "0.0.0.0".to_string();
        for iface in pnet::datalink::interfaces() {
            let iface: NetworkInterface = iface;
            if iface.ips.len() > 0 {
                let ipsv = format!("{}", iface.ips[0]);
                let ips: &str = ipsv.split('/').next().unwrap_or("");
                if format!("{}", ips).starts_with("10.") || format!("{}", ips).starts_with("192.") {
                    ip = ips.to_string();
                }
            }
        }
        if start(port).await {
            log_message(format("community_active", &[&ip, &port.to_string()]));
        } else {
            if port < 1024 {
                log_message(format("community_start_error_admin", &[&port.to_string()]));
            } else {
                log_message(format("community_start_error", &[&port.to_string()]));
            }
        }
        if !has_dir("web") {
            /*download_and_extract_zip(
                "weblink",
                "web",
            )
            .await;*/
        }
        loop {
            if *SHUTDOWN.read().await {
                break;
            }
            let omikron: Arc<OmikronConnection> = Arc::new(OmikronConnection::new());
            omikron.connect().await;
            omikron
                .send_message(
                    CommunicationValue::new(CommunicationType::identification)
                        .add_data(DataTypes::user_ids, JsonValue::String(sb.to_string()))
                        .add_data(
                            DataTypes::iota_id,
                            JsonValue::Number(Number::from(CONFIG.read().await.get_iota_id())),
                        )
                        .to_json()
                        .to_string()
                        .as_mut()
                        .to_string(),
                )
                .await;
            let mut omikron_connection = OMIKRON_CONNECTION.write().await;
            *omikron_connection = Some(omikron.clone());
            log_message_trans("setup_completed");
            loop {
                if *SHUTDOWN.read().await {
                    break;
                }
                if !omikron.is_connected().await {
                    break;
                }
                sleep(Duration::from_secs(1)).await;
            }
        }
        if *RELOAD.read().await {
            loop {
                if ACTIVE_TASKS.lock().unwrap().is_empty() {
                    break;
                }
                sleep(Duration::from_secs(1)).await;
            }
            &CONFIG.write().await.clear();
            user_manager::clear();
            community_manager::clear();
            *APP_STATE.lock().unwrap() = AppState::new();
        }
    }
}
