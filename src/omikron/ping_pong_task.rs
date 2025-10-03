use crate::APP_STATE;
use crate::data::communication::{CommunicationType, CommunicationValue, DataTypes};
use crate::gui::log_panel::{log_message, log_message_trans};
use crate::omikron::omikron_connection::OmikronConnection;
use crate::users::contact::Contact;
use crate::users::user_community_util::UserCommunityUtil;
use crate::util::chat_files::ChatFiles;
use crate::util::chats_util::{get_user, get_users, mod_user};
use futures_util::{SinkExt, StreamExt};
use json::JsonValue;
use json::number::Number;
use std::collections::HashMap;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant, sleep};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async, tungstenite::protocol::Message,
};
use uuid::Uuid;

impl OmikronConnection {
    pub async fn send_ping(&self) {
        let uuid = Uuid::new_v4();
        let send_time = Instant::now();

        self.message_send_times.lock().await.insert(uuid, send_time);
        self.send_ping_message(uuid).await;
    }

    pub async fn send_ping_message(&self, uuid: Uuid) {
        let ping_message = CommunicationValue::new(CommunicationType::ping)
            .with_id(uuid)
            .add_data_num(DataTypes::last_ping, Number::from(2))
            .to_json()
            .to_string();

        self.send_message(ping_message).await;
    }

    /// Handles incoming pong and calculates latency
    pub async fn handle_pong(&self, cv: &CommunicationValue, log: bool) {
        let id = cv.get_id();
        let send_time_opt = {
            let queue = self.message_send_times.lock().await;
            queue.get(&id).cloned()
        };

        if let Some(send_time) = send_time_opt {
            let ping = Instant::now().duration_since(send_time).as_millis() as f64;
            self.message_send_times.lock().await.remove(&id);

            if log {
                APP_STATE.lock().unwrap().push_ping_val(ping);
            }
        }
    }
}
