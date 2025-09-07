use futures_util::SinkExt;
use json::{self, JsonValue};
use json::JsonValue::String;
use uuid::Uuid;

mod data;
mod omikron;
mod util;
mod users;
mod eula;
mod gui {
    pub mod ratatui_interface;
}

mod auth;
use crate::eula::*;
use crate::omikron::omikron_connection::{OmikronConnection};
use crate::data::communication::{CommunicationValue, CommunicationType, DataTypes};
use crate::users::user_manager::UserManager;
use crate::util::config_util::ConfigUtil;

#[tokio::main]
async fn main() {
    if(!eula_checker::check_eula()){
        println!("Please accept the end user license agreement before launching!");
        return;
    }
    
    let mut c_util = ConfigUtil::new();
    c_util.load();
    if !c_util.config.has_key("iota_id") {
        c_util.change("iota_id", Uuid::new_v4());
        c_util.save();
    }

    
    UserManager::load_users().await;
    let mut sb = "".to_string();
    for up in UserManager::get_users() {
        sb = sb + "," + &*up.user_id.to_string();
    }

    UserManager::save_users();
    let omikron: OmikronConnection = OmikronConnection::new();
    omikron.connect().await;
    omikron.send_message(
        CommunicationValue::new(
            CommunicationType::Identification
        )
            .add_data(DataTypes::UserIds, String(sb.to_string()))
            .add_data(DataTypes::IotaId, String(c_util.config["iota_id"].to_string()))
            .to_json()
            .to_string()
            .as_mut()
            .to_string()
    ).await;
    
    loop {}
}
