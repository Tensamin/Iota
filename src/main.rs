use tokio::runtime::Runtime;
use std::process::{Command, ExitStatus};
use futures_util::SinkExt;
use json::{self};
use uuid::Uuid;

mod data;
mod omikron;
mod util;
mod users;
mod gui {
    pub mod ratatui_interface;
}
mod auth;

use crate::omikron::omikron_connection::{OmikronConnection};
use crate::data::communication::{CommunicationValue, CommunicationType, DataTypes};
use gui::{ratatui_interface};

#[tokio::main]
async fn main() {
    let mut omikron: OmikronConnection = OmikronConnection::new();

    omikron.connect().await;

    omikron.send_message(
        CommunicationValue::new(
            CommunicationType::Identification
        )
            .add_data(DataTypes::UserIds, json::JsonValue::String(Uuid::new_v4().to_string()))
            .add_data(DataTypes::IotaId, json::JsonValue::String(Uuid::new_v4().to_string()))
            .to_json()
            .to_string()
            .as_mut()
            .to_string()
    ).await;
    omikron.close().await;


    let mut child = Command::new("sleep").arg("2").spawn().unwrap();
    let _result = child.wait().unwrap();

    println!("reached end of main");
}
