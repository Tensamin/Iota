use crate::data::communication::{CommunicationType, CommunicationValue, DataTypes};
use crate::omikron::ping_pong_task::PingPongTask;
use crate::users::contact::Contact;
use crate::users::user_community_util::UserCommunityUtil;
use crate::util::chat_files::ChatFiles;
use crate::util::chats_util::{get_user, get_users, mod_user};
use futures_util::{SinkExt, StreamExt};
use json::JsonValue;
use json::number::Number;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};
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
    waiting: Arc<Mutex<HashMap<Uuid, Box<dyn Fn(CommunicationValue) + Send>>>>, // waiting for responses
    pingpong: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,                  // ping-pong handler
}

impl OmikronConnection {
    pub fn new() -> Self {
        Self {
            writer: Arc::new(Mutex::new(None)),
            waiting: Arc::new(Mutex::new(HashMap::new())),
            pingpong: Arc::new(Mutex::new(None)),
        }
    }

    /// Connect loop with retry
    pub async fn connect<'a>(&'a self) {
        loop {
            match connect_async("wss://tensamin.methanium.net/ws/iota/").await {
                Ok((ws_stream, _)) => {
                    let (write_half, read_half) = ws_stream.split();
                    *self.writer.lock().await = Some(write_half);
                    self.spawn_listener(read_half);
                    self.start_ping_pong_task().await;
                    break;
                }
                Err(e) => {
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    // Start the PingPongTask
    pub async fn start_ping_pong_task(&self) {
        let ping_pong_task = PingPongTask::new(Arc::new(self.clone()));

        // Store the task handle in `pingpong` so we can manage it
        let mut pingpong_handle = self.pingpong.lock().await;
        *pingpong_handle = Some(tokio::spawn(async move {
            ping_pong_task.run_ping_loop();
        }));
    }

    pub async fn send_message(&self, msg: String) {
        Self::send_message_static(&self.writer, msg).await;
    }

    pub async fn disconnect(&self) {
        if let Some(handle) = self.pingpong.lock().await.take() {
            handle.abort();
        }
        if let Some(mut ws) = self.writer.lock().await.take() {
            let _ = ws.close().await;
        }
    }

    /// Listener for all incoming messages
    fn spawn_listener(
        &self,
        mut read_half: futures_util::stream::SplitStream<
            WebSocketStream<MaybeTlsStream<TcpStream>>,
        >,
    ) {
        let waiting = self.waiting.clone();
        let writer = self.writer.clone();

        tokio::spawn(async move {
            while let Some(msg) = read_half.next().await {
                match msg {
                    Ok(Message::Close(Some(frame))) => {
                        println!("[Omikron] Closed: {:?}", frame);
                        break;
                    }
                    Ok(Message::Text(text)) => {
                        let mut cv = CommunicationValue::from_json(&text); // needs CommunicationValue parser
                        if cv.is_type(CommunicationType::Pong) {
                            // handle pingpong reset here
                            continue;
                        }
                        // ************************************************ //
                        // Direct messages                                  //
                        // ************************************************ //
                        println!("[Omikron] Received message: {:?}", cv);
                        if cv.is_type(CommunicationType::MessageOtherIota) {
                            let sender_id = &cv.get_sender();
                            let receiver_id = &cv.get_receiver();
                            ChatFiles::add_message(
                                cv.get_data(DataTypes::SendTime)
                                    .unwrap()
                                    .as_i64()
                                    .unwrap_or(-1),
                                false,
                                *receiver_id,
                                *sender_id,
                                cv.get_data(DataTypes::MessageContent)
                                    .unwrap()
                                    .as_str()
                                    .unwrap(),
                            );
                            let response = CommunicationValue::new(CommunicationType::MessageLive)
                                .with_id(cv.get_id())
                                .with_receiver(cv.get_receiver())
                                .add_data(
                                    DataTypes::SendTime,
                                    cv.get_data(DataTypes::SendTime).unwrap().clone(),
                                )
                                .add_data(
                                    DataTypes::Message,
                                    cv.get_data(DataTypes::MessageContent).unwrap().clone(),
                                )
                                .add_data(
                                    DataTypes::SenderId,
                                    JsonValue::String(cv.get_sender().clone().to_string()),
                                );
                            Self::send_message_static(
                                &writer.clone(),
                                response.to_json().to_string(),
                            )
                            .await;
                            continue;
                        }

                        if cv.is_type(CommunicationType::Message) {
                            let my_id = cv.get_sender();
                            ChatFiles::add_message(
                                SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis() as i64,
                                true,
                                my_id,
                                Uuid::from_str(
                                    &*cv.get_data(DataTypes::ReceiverId).unwrap().to_string(),
                                )
                                .unwrap(),
                                &*cv.get_data(DataTypes::MessageContent).unwrap().to_string(),
                            );
                            // ack
                            let ack = CommunicationValue::ack_message(cv.get_id(), my_id);
                            Self::send_message_static(&writer.clone(), ack.to_json().to_string())
                                .await;
                            // forward
                            let forward = CommunicationValue::forward_to_other_iota(&mut cv);
                            Self::send_message_static(
                                &writer.clone(),
                                forward.to_json().to_string(),
                            )
                            .await;
                            continue;
                        }

                        if cv.is_type(CommunicationType::MessageGet) {
                            let my_id = cv.get_sender();
                            let partner_id = Uuid::from_str(
                                &*cv.get_data(DataTypes::ChatPartnerId).unwrap().to_string(),
                            )
                            .unwrap();
                            let offset = cv
                                .get_data(DataTypes::LoadedMessages)
                                .unwrap_or(&JsonValue::Null)
                                .to_string()
                                .parse::<i64>()
                                .unwrap_or(0);
                            let amount = cv
                                .get_data(DataTypes::MessageAmount)
                                .unwrap_or(&JsonValue::Null)
                                .to_string()
                                .parse::<i64>()
                                .unwrap_or(0);

                            let messages =
                                ChatFiles::get_messages(my_id, partner_id, offset, amount); // needs ChatFiles
                            let mut resp = CommunicationValue::new(CommunicationType::MessageChunk)
                                .with_id(cv.get_id())
                                .with_receiver(my_id);
                            if !messages.is_empty() {
                                resp = resp.add_data(DataTypes::MessageChunk, messages);
                            }
                            Self::send_message_static(&writer.clone(), resp.to_json().to_string())
                                .await;
                            continue;
                        }

                        if cv.is_type(CommunicationType::GetChats) {
                            let user_id = cv.get_sender();
                            let users = get_users(user_id); // needs ChatsUtil
                            let resp = CommunicationValue::new(CommunicationType::GetChats)
                                .with_id(cv.get_id())
                                .with_receiver(user_id)
                                .add_data(DataTypes::UserIds, users);
                            println!("ALARM: {}", resp.to_json().to_string());
                            Self::send_message_static(&writer.clone(), resp.to_json().to_string())
                                .await;
                            continue;
                        }

                        if cv.is_type(CommunicationType::AddChat) {
                            let user_id = cv.get_sender();
                            let other_id = Uuid::from_str(
                                &*cv.get_data(DataTypes::UserId).unwrap().to_string(),
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
                            let resp = CommunicationValue::new(CommunicationType::AddChat)
                                .with_id(cv.get_id())
                                .with_receiver(user_id);
                            Self::send_message_static(&writer.clone(), resp.to_json().to_string())
                                .await;
                            continue;
                        }

                        if cv.is_type(CommunicationType::AddCommunity) {
                            UserCommunityUtil::add_community(
                                cv.get_sender(),
                                cv.get_data(DataTypes::CommunityAddress)
                                    .unwrap()
                                    .to_string(),
                                cv.get_data(DataTypes::CommunityTitle).unwrap().to_string(),
                                cv.get_data(DataTypes::Position).unwrap().to_string(),
                            );
                            let resp = CommunicationValue::new(CommunicationType::AddCommunity)
                                .with_id(cv.get_id())
                                .with_receiver(cv.get_sender());
                            Self::send_message_static(&writer.clone(), resp.to_json().to_string())
                                .await;
                            continue;
                        }

                        if cv.is_type(CommunicationType::GetCommunities) {
                            let resp = CommunicationValue::new(CommunicationType::GetCommunities)
                                .with_id(cv.get_id())
                                .with_receiver(cv.get_sender())
                                .add_data(
                                    DataTypes::Communities,
                                    UserCommunityUtil::get_communities(cv.get_sender()),
                                ); // needs UserCommunityUtil
                            Self::send_message_static(&writer.clone(), resp.to_json().to_string())
                                .await;
                            continue;
                        }

                        if cv.is_type(CommunicationType::RemoveCommunity) {
                            UserCommunityUtil::remove_community(
                                cv.get_sender(),
                                cv.get_data(DataTypes::CommunityAddress)
                                    .unwrap()
                                    .to_string(),
                            ); // needs UserCommunityUtil
                            let resp = CommunicationValue::new(CommunicationType::RemoveCommunity)
                                .with_id(cv.get_id())
                                .with_receiver(cv.get_sender());
                            Self::send_message_static(&writer.clone(), resp.to_json().to_string())
                                .await;
                            continue;
                        }
                    }
                    Err(e) => {
                        eprintln!("[Omikron] Error: {}", e);
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
    ) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        let mut guard = writer.lock().await;
        if let Some(writer) = guard.as_mut() {
            writer.send(Message::Text(msg)).await?;
            writer.flush().await?;
            Ok(())
        } else {
            Err(tokio_tungstenite::tungstenite::Error::ConnectionClosed)
        }
    }

    pub fn on_answer<F>(&self, message_id: Uuid, callback: F)
    where
        F: Fn(CommunicationValue) + Send + 'static,
    {
        tokio::spawn({
            let waiting = self.waiting.clone();
            async move {
                waiting.lock().await.insert(message_id, Box::new(callback));
            }
        });
    }

    pub async fn send_ping_message(&self, uuid: Uuid) {
        // Send the ping message over the connection
        let ping_message = CommunicationValue::new(CommunicationType::Ping)
            .with_id(uuid)
            .add_data_num(DataTypes::LastPing, Number::from(2))
            .to_json()
            .to_string();
        self.send_message(ping_message).await;
    }
    pub async fn reconnect(&self) {
        self.disconnect().await;
        self.connect().await;
    }
}
