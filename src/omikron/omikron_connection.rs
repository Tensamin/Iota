use crate::auth::local_auth;
use crate::data::communication::{CommunicationType, CommunicationValue, DataTypes};
use crate::gui::log_panel::{log_cv, log_message, log_message_trans};
use crate::langu::language_manager::format;
use crate::users::contact::Contact;
use crate::users::user_community_util::UserCommunityUtil;
use crate::util::chat_files;
use crate::util::chats_util::{get_user, get_users, mod_user};
use crate::util::file_util::{get_children, load_file, save_file};
use futures::Stream;
use futures::stream::{SplitSink, SplitStream};
use futures_util::sink::Sink;
use futures_util::{SinkExt, StreamExt};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use json::JsonValue;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, LazyLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock};
use tokio::time::{Duration, Instant, sleep};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tungstenite::Utf8Bytes;
use uuid::Uuid;
pub static OMIKRON_CONNECTION: LazyLock<Arc<RwLock<Option<Arc<OmikronConnection>>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionVariant {
    Omikron,
    ClientUnauthenticated,
    ClientAuthenticated,
}

#[derive(Clone)]
pub struct OmikronConnection {
    pub variant: Arc<RwLock<ConnectionVariant>>,
    pub user_id: Arc<RwLock<Option<Uuid>>>,
    pub(crate) writer:
        Arc<Mutex<Option<Box<dyn Sink<Message, Error = tungstenite::Error> + Send + Unpin>>>>,
    waiting: Arc<Mutex<HashMap<Uuid, Box<dyn Fn(CommunicationValue) + Send + Sync>>>>, // waiting for responses
    pingpong: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>, // ping-pong handler
    pub last_ping: Arc<Mutex<i64>>,
    pub message_send_times: Arc<Mutex<HashMap<Uuid, Instant>>>,
    pub is_connected: Arc<Mutex<bool>>,
}

impl OmikronConnection {
    pub fn new() -> Self {
        Self {
            variant: Arc::new(RwLock::new(ConnectionVariant::Omikron)),
            user_id: Arc::new(RwLock::new(None)),
            writer: Arc::new(Mutex::new(None)),
            waiting: Arc::new(Mutex::new(HashMap::new())),
            pingpong: Arc::new(Mutex::new(None)),
            last_ping: Arc::new(Mutex::new(-1)),
            message_send_times: Arc::new(Mutex::new(HashMap::new())),
            is_connected: Arc::new(Mutex::new(false)),
        }
    }
    pub async fn client(
        writer: SplitSink<tokio_tungstenite::WebSocketStream<TokioIo<Upgraded>>, Message>,
        reader: SplitStream<tokio_tungstenite::WebSocketStream<TokioIo<Upgraded>>>,
    ) -> Arc<Self> {
        let connection = Arc::new(Self {
            variant: Arc::new(RwLock::new(ConnectionVariant::ClientUnauthenticated)),
            user_id: Arc::new(RwLock::new(None)),
            writer: Arc::new(Mutex::new(Some(Box::new(writer)
                as Box<dyn Sink<Message, Error = tungstenite::Error> + Send + Unpin>))),
            waiting: Arc::new(Mutex::new(HashMap::new())),
            pingpong: Arc::new(Mutex::new(None)),
            last_ping: Arc::new(Mutex::new(-1)),
            message_send_times: Arc::new(Mutex::new(HashMap::new())),
            is_connected: Arc::new(Mutex::new(false)),
        });
        let boxed_reader: Box<
            dyn Stream<Item = Result<Message, tungstenite::Error>> + Send + Unpin,
        > = Box::new(reader);

        connection.spawn_listener(boxed_reader).await;
        connection
    }
    pub async fn is_connected(&self) -> bool {
        *self.is_connected.lock().await
    }
    /// Connect loop with retry
    pub async fn connect(self: &Arc<Self>) {
        loop {
            match connect_async("wss://app.tensamin.net/ws/iota/").await {
                Ok((ws_stream, _)) => {
                    let (write_half, read_half) = ws_stream.split();
                    *self.writer.lock().await = Some(Box::new(write_half));
                    let boxed_reader: Box<
                        dyn Stream<Item = Result<Message, tungstenite::Error>> + Send + Unpin,
                    > = Box::new(read_half);
                    self.clone().spawn_listener(boxed_reader).await;
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
                Err(e) => {
                    log_message(format("connection_failed", &[&e.to_string().as_str()]));
                    *self.is_connected.lock().await = false;
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }
    pub async fn send_message(&self, msg: String) {
        Self::send_message_static(&self.writer, msg).await
    }

    pub async fn set_variant(self: &Arc<Self>, variant: ConnectionVariant) {
        *self.variant.write().await = variant;
    }
    pub async fn set_user_id(self: &Arc<Self>, user_id: Uuid) {
        *self.user_id.write().await = Some(user_id);
    }

    /// Listener for all incoming messages
    async fn spawn_listener(
        self: &Arc<Self>,
        mut read_half: Box<dyn Stream<Item = Result<Message, tungstenite::Error>> + Send + Unpin>,
    ) {
        let waiting_out = self.waiting.clone();
        let writer_out = self.writer.clone();
        let is_connected_out = self.is_connected.clone();
        let sel_out = self.clone();
        let variant = self.variant.clone();
        let sel_arc_out = self.clone();
        tokio::spawn(async move {
            while let Some(msg) = read_half.next().await {
                let waiting = waiting_out.clone();
                let writer = writer_out.clone();
                let is_connected = is_connected_out.clone();
                let sel = sel_out.clone();
                let variant = variant.clone();
                let sel_arc = sel_arc_out.clone();
                tokio::spawn(async move {
                    match msg {
                        Ok(Message::Close(Some(frame))) => {
                            log_message(format!("[Omikron] Closed: {:?}", frame));
                            *is_connected.lock().await = false;
                            return;
                        }
                        Ok(Message::Text(text)) => {
                            let mut cv = CommunicationValue::from_json(&text);
                            if cv.is_type(CommunicationType::pong) {
                                sel.handle_pong(&cv, true).await;
                                return;
                            }
                            let com = variant.read().await.clone();
                            if com == ConnectionVariant::ClientUnauthenticated {
                                if cv.is_type(CommunicationType::identification) {
                                    // Extract user ID
                                    let user_id = match cv.get_data(DataTypes::user_id) {
                                        Some(id_str) => {
                                            match Uuid::parse_str(&id_str.to_string()) {
                                                Ok(id) => id,
                                                Err(_) => {
                                                    sel_arc.send_message(
                                                        CommunicationValue::new(CommunicationType::error_invalid_user_id)
                                                            .with_id(cv.get_id())
                                                            .to_json()
                                                            .to_string()
                                                    )
                                                    .await;
                                                    return;
                                                }
                                            }
                                        }
                                        None => {
                                            sel_arc
                                                .send_message(
                                                    CommunicationValue::new(
                                                        CommunicationType::error_invalid_user_id,
                                                    )
                                                    .with_id(cv.get_id())
                                                    .to_json()
                                                    .to_string(),
                                                )
                                                .await;
                                            return;
                                        }
                                    };

                                    // Validate private key
                                    if let Some(private_key_hash) =
                                        cv.get_data(DataTypes::private_key_hash)
                                    {
                                        log_message(format!(
                                            "private_key_hash: {}",
                                            private_key_hash
                                        ));
                                        let is_valid = local_auth::is_private_key_valid(
                                            &user_id,
                                            &private_key_hash.to_string(),
                                        );

                                        if !is_valid {
                                            log_message("Invalid private key");
                                            sel_arc.send_message(
                                                CommunicationValue::new(
                                                    CommunicationType::error_invalid_private_key,
                                                )
                                                .with_id(cv.get_id())
                                                .to_json()
                                                .to_string(),
                                            )
                                            .await;
                                            return;
                                        }
                                    } else {
                                        log_message("Missing private key");
                                        sel_arc
                                            .send_message(
                                                CommunicationValue::new(
                                                    CommunicationType::error_invalid_private_key,
                                                )
                                                .with_id(cv.get_id())
                                                .to_json()
                                                .to_string(),
                                            )
                                            .await;
                                        return;
                                    }

                                    // Set identification data

                                    sel_arc.set_user_id(user_id).await;
                                    sel_arc
                                        .set_variant(ConnectionVariant::ClientAuthenticated)
                                        .await;

                                    let response = CommunicationValue::new(
                                        CommunicationType::identification_response,
                                    )
                                    .with_id(cv.get_id());
                                    sel_arc.send_message(response.to_json().to_string()).await;
                                }
                            }
                            // ************************************************ //
                            // Direct messages                                  //
                            // ************************************************ //
                            log_cv(&cv);
                            if let Some(x) = waiting.lock().await.remove(&cv.get_id()) {
                                x(cv);
                                return;
                            }
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
                                let response =
                                    CommunicationValue::new(CommunicationType::message_live)
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
                                return;
                            }

                            if cv.is_type(CommunicationType::message_send) {
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
                                let ack = CommunicationValue::new(CommunicationType::message)
                                    .with_id(cv.get_id())
                                    .with_receiver(my_id);
                                Self::send_message_static(
                                    &writer.clone(),
                                    ack.to_json().to_string(),
                                )
                                .await;
                                let forward = CommunicationValue::forward_to_other_iota(&mut cv);
                                Self::send_message_static(
                                    &writer.clone(),
                                    forward.to_json().to_string(),
                                )
                                .await;
                                return;
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

                                Self::send_message_static(
                                    &writer.clone(),
                                    resp.to_json().to_string(),
                                )
                                .await;
                                return;
                            }

                            if cv.is_type(CommunicationType::get_chats) {
                                let user_id = cv.get_sender().unwrap();
                                let users = get_users(user_id);
                                let resp = CommunicationValue::new(CommunicationType::get_chats)
                                    .with_id(cv.get_id())
                                    .with_receiver(user_id)
                                    .add_data(DataTypes::user_ids, users);
                                Self::send_message_static(
                                    &writer.clone(),
                                    resp.to_json().to_string(),
                                )
                                .await;
                                return;
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
                                Self::send_message_static(
                                    &writer.clone(),
                                    resp.to_json().to_string(),
                                )
                                .await;
                                return;
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
                                let resp =
                                    CommunicationValue::new(CommunicationType::add_community)
                                        .with_id(cv.get_id())
                                        .with_receiver(cv.get_sender().unwrap());
                                Self::send_message_static(
                                    &writer.clone(),
                                    resp.to_json().to_string(),
                                )
                                .await;
                                return;
                            }

                            if cv.is_type(CommunicationType::get_communities) {
                                let resp =
                                    CommunicationValue::new(CommunicationType::get_communities)
                                        .with_id(cv.get_id())
                                        .with_receiver(cv.get_sender().unwrap())
                                        .add_array(
                                            DataTypes::communities,
                                            UserCommunityUtil::get_communities(
                                                cv.get_sender().unwrap(),
                                            ),
                                        );
                                Self::send_message_static(
                                    &writer.clone(),
                                    resp.to_json().to_string(),
                                )
                                .await;
                                return;
                            }

                            if cv.is_type(CommunicationType::remove_community) {
                                UserCommunityUtil::remove_community(
                                    cv.get_sender().unwrap(),
                                    cv.get_data(DataTypes::community_address)
                                        .unwrap()
                                        .to_string(),
                                ); // needs UserCommunityUtil
                                let resp =
                                    CommunicationValue::new(CommunicationType::remove_community)
                                        .with_id(cv.get_id())
                                        .with_receiver(cv.get_sender().unwrap());
                                Self::send_message_static(
                                    &writer.clone(),
                                    resp.to_json().to_string(),
                                )
                                .await;
                                return;
                            }

                            if cv.is_type(CommunicationType::settings_save) {
                                let my_id = cv.get_sender().unwrap();
                                let settings_name =
                                    cv.get_data(DataTypes::settings_name).unwrap().to_string();
                                let settings_value =
                                    cv.get_data(DataTypes::payload).unwrap().to_string();

                                save_file(
                                    &format!("users/{}/settings/", my_id),
                                    &format!("{}.settings", settings_name),
                                    &settings_value,
                                );

                                let response =
                                    CommunicationValue::new(CommunicationType::settings_save)
                                        .with_receiver(my_id)
                                        .with_id(cv.get_id());

                                Self::send_message_static(
                                    &writer.clone(),
                                    response.to_json().to_string(),
                                )
                                .await;
                                return;
                            }
                            if cv.is_type(CommunicationType::settings_load) {
                                let my_id = cv.get_sender().unwrap();
                                let settings_name =
                                    cv.get_data(DataTypes::settings_name).unwrap().to_string();
                                let settings_value_str = load_file(
                                    &format!("users/{}/settings/", my_id),
                                    &format!("{}.settings", settings_name),
                                );
                                let settings_value_json = JsonValue::from(settings_value_str);
                                let response =
                                    CommunicationValue::new(CommunicationType::settings_load)
                                        .with_id(cv.get_id())
                                        .with_receiver(my_id)
                                        .add_data(DataTypes::payload, settings_value_json)
                                        .add_data_str(DataTypes::settings_name, settings_name);

                                Self::send_message_static(
                                    &writer.clone(),
                                    response.to_json().to_string(),
                                )
                                .await;
                                return;
                            }
                            if cv.is_type(CommunicationType::settings_list) {
                                let my_id = cv.get_sender().unwrap();
                                let settings = get_children(&format!("users/{}/settings/", my_id));
                                let mut settings_json = JsonValue::new_array();
                                for s in settings {
                                    let s = s.replace(".settings", "");
                                    if s.is_empty() {
                                        continue;
                                    }
                                    let _ = settings_json.push(JsonValue::String(s));
                                }
                                let response =
                                    CommunicationValue::new(CommunicationType::settings_list)
                                        .with_id(cv.get_id())
                                        .with_receiver(my_id)
                                        .add_data(DataTypes::settings, settings_json);

                                Self::send_message_static(
                                    &writer.clone(),
                                    response.to_json().to_string(),
                                )
                                .await;
                                return;
                            }
                        }
                        Err(e) => {
                            log_message(format!("[Omikron] Error: {}", e));
                            *is_connected.lock().await = false;
                            return;
                        }
                        _ => {}
                    }
                });
            }
        });
    }
    pub async fn send_message_static(
        writer: &Arc<
            Mutex<Option<Box<dyn Sink<Message, Error = tungstenite::Error> + Send + Unpin>>>,
        >,
        msg: String,
    ) {
        let mut guard = writer.lock().await;
        if let Some(writer) = guard.as_mut() {
            if let Ok(_) = writer.send(Message::Text(Utf8Bytes::from(msg))).await {
                if let Ok(_) = writer.flush().await {
                    return;
                }
            }
        }
        log_message_trans("send_message_failed");
    }
}
