use crate::data::communication::{CommunicationType, CommunicationValue, DataTypes};
use crate::gui::log_panel::{log_cv, log_message};
use crate::users::contact::Contact;
use crate::users::user_community_util::UserCommunityUtil;
use crate::util::chat_files;
use crate::util::chats_util::{get_user, get_users, mod_user};
use futures_util::{SinkExt, StreamExt};
use json::JsonValue;
use std::collections::HashMap;
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

#[derive(Clone)]
pub struct OmikronConnection {
    pub(crate) writer: Arc<
        Mutex<
            Option<
                futures_util::stream::SplitSink<
                    WebSocketStream<MaybeTlsStream<TcpStream>>,
                    Message,
                >,
            >,
        >,
    >,
    waiting: Arc<Mutex<HashMap<Uuid, Box<dyn Fn(CommunicationValue) + Send + Sync>>>>, // waiting for responses
    pingpong: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>, // ping-pong handler
    pub last_ping: Arc<Mutex<i64>>,
    pub message_send_times: Arc<Mutex<HashMap<Uuid, Instant>>>,
    pub is_connected: Arc<Mutex<bool>>,
}

impl OmikronConnection {
    pub fn new() -> Self {
        Self {
            writer: Arc::new(Mutex::new(None)),
            waiting: Arc::new(Mutex::new(HashMap::new())),
            pingpong: Arc::new(Mutex::new(None)),
            last_ping: Arc::new(Mutex::new(-1)),
            message_send_times: Arc::new(Mutex::new(HashMap::new())),
            is_connected: Arc::new(Mutex::new(false)),
        }
    }
    pub async fn is_connected(&self) -> bool {
        *self.is_connected.lock().await
    }
    /// Connect loop with retry
    pub async fn connect(&self) {
        loop {
            match connect_async("wss://tensamin.methanium.net/ws/iota/").await {
                Ok((ws_stream, _)) => {
                    let (write_half, read_half) = ws_stream.split();
                    *self.writer.lock().await = Some(write_half);
                    self.spawn_listener(read_half).await;
                    let cloned_self = self.clone();
                    let handle = tokio::spawn(async move {
                        loop {
                            cloned_self.send_ping().await;
                            sleep(Duration::from_secs(1)).await;
                        }
                    });

                    *self.is_connected.lock().await = true;
                    *self.pingpong.lock().await = Some(handle);
                    break;
                }
                Err(_) => {
                    log_message("CONNECTION FAILED");
                    *self.is_connected.lock().await = false;
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }
    pub async fn send_message(&self, msg: String) {
        Self::send_message_static(&self.writer, msg).await;
    }

    /// Listener for all incoming messages
    async fn spawn_listener(
        &self,
        mut read_half: futures_util::stream::SplitStream<
            WebSocketStream<MaybeTlsStream<TcpStream>>,
        >,
    ) {
        let waiting = self.waiting.clone();
        let writer = self.writer.clone();
        let is_connected = self.is_connected.clone();
        let sel = self.clone();
        tokio::spawn(async move {
            while let Some(msg) = read_half.next().await {
                match msg {
                    Ok(Message::Close(Some(frame))) => {
                        log_message(format!("[Omikron] Closed: {:?}", frame));
                        *is_connected.lock().await = false;
                        break;
                    }
                    Ok(Message::Text(text)) => {
                        let mut cv = CommunicationValue::from_json(&text);
                        if cv.is_type(CommunicationType::pong) {
                            sel.handle_pong(&cv, true).await;
                            continue;
                        }
                        // ************************************************ //
                        // Direct messages                                  //
                        // ************************************************ //
                        log_cv(&cv);
                        if cv.is_type(CommunicationType::message_other_iota) {
                            let sender_id = &cv.get_sender().unwrap();
                            let receiver_id = &cv.get_receiver().unwrap();

                            chat_files::add_message(
                                cv.get_data(DataTypes::send_time)
                                    .unwrap()
                                    .as_i64()
                                    .unwrap_or(0) as u128,
                                false,
                                *receiver_id,
                                *sender_id,
                                cv.get_data(DataTypes::content).unwrap().as_str().unwrap(),
                            );
                            let response = CommunicationValue::new(CommunicationType::message_live)
                                .with_id(cv.get_id())
                                .with_receiver(cv.get_receiver().unwrap())
                                .add_data(
                                    DataTypes::send_time,
                                    cv.get_data(DataTypes::send_time).unwrap().clone(),
                                )
                                .add_data(
                                    DataTypes::message,
                                    cv.get_data(DataTypes::content).unwrap().clone(),
                                )
                                .add_data(
                                    DataTypes::sender_id,
                                    JsonValue::String(cv.get_sender().unwrap().to_string()),
                                );
                            Self::send_message_static(
                                &writer.clone(),
                                response.to_json().to_string(),
                            )
                            .await;
                            continue;
                        }

                        if cv.is_type(CommunicationType::message_send) {
                            /* DATA CONTAINER:
                            "sent_by_self": true,
                            "timestamp": unixTimestamp,
                            "files": [ // wenn keine files dann weglassen
                                {
                                    "name": "<name>",
                                    "id": "<uuid>",
                                    "type": "[ image | image_top_right | file ]"
                                }
                            ],
                            "content": "<enc markdown>"
                            */
                            let my_id = cv.get_sender().unwrap();
                            let other_id = Uuid::from_str(
                                &*cv.get_data(DataTypes::receiver_id).unwrap().to_string(),
                            )
                            .unwrap();
                            chat_files::add_message(
                                SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis() as u128,
                                true,
                                my_id,
                                other_id,
                                &*cv.get_data(DataTypes::content).unwrap().to_string(),
                            );
                            let ack = CommunicationValue::ack_message(cv.get_id(), my_id);
                            Self::send_message_static(&writer.clone(), ack.to_json().to_string())
                                .await;
                            let forward = CommunicationValue::forward_to_other_iota(&mut cv);
                            Self::send_message_static(
                                &writer.clone(),
                                forward.to_json().to_string(),
                            )
                            .await;
                            continue;
                        }

                        if cv.is_type(CommunicationType::messages_get) {
                            let my_id = cv.get_sender().unwrap();
                            let partner_id = Uuid::from_str(
                                &*cv.get_data(DataTypes::user_id).unwrap().to_string(),
                            )
                            .unwrap();
                            let offset = cv
                                .get_data(DataTypes::offset)
                                .unwrap_or(&JsonValue::Null)
                                .to_string()
                                .parse::<i64>()
                                .unwrap_or(0);
                            let amount = cv
                                .get_data(DataTypes::amount)
                                .unwrap_or(&JsonValue::Null)
                                .to_string()
                                .parse::<i64>()
                                .unwrap_or(0);
                            let messages =
                                chat_files::get_messages(my_id, partner_id, offset, amount);
                            let resp = CommunicationValue::new(CommunicationType::messages_get)
                                .with_id(cv.get_id())
                                .with_receiver(my_id)
                                .add_data(DataTypes::messages, messages);

                            Self::send_message_static(&writer.clone(), resp.to_json().to_string())
                                .await;
                            continue;
                        }

                        if cv.is_type(CommunicationType::get_chats) {
                            let user_id = cv.get_sender().unwrap();
                            let users = get_users(user_id);
                            let resp = CommunicationValue::new(CommunicationType::get_chats)
                                .with_id(cv.get_id())
                                .with_receiver(user_id)
                                .add_data(DataTypes::user_ids, users);
                            Self::send_message_static(&writer.clone(), resp.to_json().to_string())
                                .await;
                            continue;
                        }

                        if cv.is_type(CommunicationType::add_chat) {
                            let user_id = cv.get_sender().unwrap();
                            let other_id = Uuid::from_str(
                                &*cv.get_data(DataTypes::user_id).unwrap().to_string(),
                            )
                            .unwrap();
                            let mut contact =
                                get_user(user_id, other_id).unwrap_or(Contact::new(other_id)); // needs ChatsUtil + Contact
                            contact.set_last_message_at(
                                SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis() as i64,
                            );
                            mod_user(user_id, &contact);
                            let resp = CommunicationValue::new(CommunicationType::add_chat)
                                .with_id(cv.get_id())
                                .with_receiver(user_id);
                            Self::send_message_static(&writer.clone(), resp.to_json().to_string())
                                .await;
                            continue;
                        }

                        if cv.is_type(CommunicationType::add_community) {
                            UserCommunityUtil::add_community(
                                cv.get_sender().unwrap(),
                                cv.get_data(DataTypes::community_address)
                                    .unwrap()
                                    .to_string(),
                                cv.get_data(DataTypes::community_title).unwrap().to_string(),
                                cv.get_data(DataTypes::position).unwrap().to_string(),
                            );
                            let resp = CommunicationValue::new(CommunicationType::add_community)
                                .with_id(cv.get_id())
                                .with_receiver(cv.get_sender().unwrap());
                            Self::send_message_static(&writer.clone(), resp.to_json().to_string())
                                .await;
                            continue;
                        }

                        if cv.is_type(CommunicationType::get_communities) {
                            let resp = CommunicationValue::new(CommunicationType::get_communities)
                                .with_id(cv.get_id())
                                .with_receiver(cv.get_sender().unwrap())
                                .add_array(
                                    DataTypes::communities,
                                    UserCommunityUtil::get_communities(cv.get_sender().unwrap()),
                                );
                            Self::send_message_static(&writer.clone(), resp.to_json().to_string())
                                .await;
                            continue;
                        }

                        if cv.is_type(CommunicationType::remove_community) {
                            UserCommunityUtil::remove_community(
                                cv.get_sender().unwrap(),
                                cv.get_data(DataTypes::community_address)
                                    .unwrap()
                                    .to_string(),
                            ); // needs UserCommunityUtil
                            let resp = CommunicationValue::new(CommunicationType::remove_community)
                                .with_id(cv.get_id())
                                .with_receiver(cv.get_sender().unwrap());
                            Self::send_message_static(&writer.clone(), resp.to_json().to_string())
                                .await;
                            continue;
                        }
                    }
                    Err(e) => {
                        log_message(format!("[Omikron] Error: {}", e));
                        *is_connected.lock().await = false;
                        break;
                    }
                    _ => {}
                }
            }
        });
    }
    pub async fn send_message_static(
        writer: &Arc<
            Mutex<
                Option<
                    futures_util::stream::SplitSink<
                        WebSocketStream<MaybeTlsStream<TcpStream>>,
                        Message,
                    >,
                >,
            >,
        >,
        msg: String,
    ) {
        let mut guard = writer.lock().await;
        if let Some(writer) = guard.as_mut() {
            writer.send(Message::Text(msg)).await;
            writer.flush().await;
        }
    }
}
