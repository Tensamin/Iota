use std::process::{Command, ExitStatus};
use json::{
    self
};
use uuid::Uuid;

mod data;
mod omikron;
mod util;
mod users;
mod auth;

use crate::auth::auth_connector::AuthConnector;
use crate::omikron::omikron_connection::{OmikronConnection};
use crate::data::communication::{CommunicationValue, CommunicationType, DataTypes};

#[tokio::main]
async fn main() {
    let alois = AuthConnector::get_uuid("aloisianer").await;
    if alois.is_none() {
        println!("No UUID");
        return
    }
    println!("{}", alois.unwrap());
    let omikron = OmikronConnection::new();
    omikron.connect().await;

    OmikronConnection::send_message_static(
        &omikron.writer,
        CommunicationValue::new(
            CommunicationType::Identification
        )
            .add_data(DataTypes::UserIds, Uuid::new_v4().to_string())
            .add_data(DataTypes::IotaId, Uuid::new_v4().to_string())
            .to_json()
            .to_string()
            .as_mut().parse().unwrap()
    ).await;
    
    
    let mut child = Command::new("sleep").arg("2").spawn().unwrap();
    let _result = child.wait().unwrap();
    
    println!("reached end of main");
}
