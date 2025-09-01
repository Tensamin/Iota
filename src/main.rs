use json;
use tokio::runtime::Runtime;
use uuid::Uuid;

mod data;
mod omikron;
mod util;
mod users;

use crate::omikron::omikron_connection::{OmikronConnection};
use crate::data::communication::{CommunicationValue, CommunicationType, DataTypes};

fn main() {
    let runtime: Runtime = tokio::runtime::Runtime::new().unwrap();
    let omikron: OmikronConnection = OmikronConnection::new();
    runtime.block_on(async {
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
    });
}
