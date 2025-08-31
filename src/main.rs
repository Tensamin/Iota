use std::time::Duration;
use json::{self, JsonValue};
use tokio::time::sleep;
use uuid::Uuid;

mod data;
mod omikron;
mod util;
mod users;

use crate::omikron::omikronConnection::{OmikronConnection};
use crate::data::communication::{CommunicationValue, LogLevel, LogValue, CommunicationType, DataTypes};

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let omikron = OmikronConnection::new();
    rt.block_on(async {
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
        ).await;
        omikron.close().await;
    });
}
