use json::{self, JsonValue};
use uuid::Uuid;

mod data;
mod omikron;

use crate::omikron::omikronConnection::{OmikronConnection};
use crate::data::communication::{CommunicationValue, LogLevel, LogValue, CommunicationType, DataTypes};

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let omikron = OmikronConnection::new();
    rt.block_on(async {
        omikron.connect().await;
    });
}
