use crate::auth::local_auth;
use crate::data::communication::{CommunicationType, CommunicationValue, DataTypes};
use crate::gui::log_panel::{log_cv, log_message, log_message_format};
use crate::users::contact::Contact;
use crate::users::user_community_util::UserCommunityUtil;
use crate::util::chat_files;
use crate::util::chats_util::{get_user, get_users, mod_user};
use crate::util::file_util::{get_children, load_file, save_file};
use crate::{ACTIVE_TASKS, SHUTDOWN};
use futures::Stream;
use futures::stream::{SplitSink, SplitStream};
use futures_util::sink::Sink;
use futures_util::{SinkExt, StreamExt};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use json::JsonValue;
use json::number::Number;
use ratatui::crossterm::event::poll;
use std::collections::HashMap;
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
    pub user_id: Arc<RwLock<i64>>,
    pub(crate) writer:
        Arc<Mutex<Option<Box<dyn Sink<Message, Error = tungstenite::Error> + Send + Unpin>>>>,
    waiting: Arc<Mutex<HashMap<Uuid, Box<dyn Fn(CommunicationValue) + Send + Sync>>>>,
    pingpong: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pub last_ping: Arc<Mutex<i64>>,
    pub message_send_times: Arc<Mutex<HashMap<Uuid, Instant>>>,
    pub is_connected: Arc<Mutex<bool>>,
}

impl OmikronConnection {
    pub fn new() -> Self {
        Self {
            variant: Arc::new(RwLock::new(ConnectionVariant::Omikron)),
            user_id: Arc::new(RwLock::new(0)),
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
            user_id: Arc::new(RwLock::new(0)),
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
            if *SHUTDOWN.read().await {
                break;
            }
            match connect_async("wss://app.tensamin.net/ws/iota/").await {
                Ok((ws_stream, _)) => {
                    let (write_half, read_half) = ws_stream.split();
                    *self.writer.lock().await = Some(Box::new(write_half));
                    let boxed_reader: Box<
                        dyn Stream<Item = Result<Message, tungstenite::Error>> + Send + Unpin,
                    > = Box::new(read_half);
                    self.clone().spawn_listener(boxed_reader).await;
                    let cloned_self = self.clone();

                    {
                        ACTIVE_TASKS.lock().unwrap().push("PingPong".to_string());
                    }
                    let handle = tokio::spawn(async move {
                        loop {
                            if *SHUTDOWN.read().await {
                                break;
                            }
                            if *cloned_self.is_connected.lock().await == false {
                                break;
                            }
                            cloned_self.send_ping().await;
                            sleep(Duration::from_secs(1)).await;
                        }
                    });
                    {
                        ACTIVE_TASKS
                            .lock()
                            .unwrap()
                            .retain(|t| !t.eq(&"PingPong".to_string()));
                    }

                    *self.is_connected.lock().await = true;
                    *self.pingpong.lock().await = Some(handle);
                    break;
                }
                Err(_) => {
                    *self.is_connected.lock().await = false;
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }
    pub async fn send_message(&self, msg: String) {
        Self::send_message_static(&self.writer, Arc::clone(&self.is_connected), msg).await;
    }

    pub async fn set_variant(self: &Arc<Self>, variant: ConnectionVariant) {
        *self.variant.write().await = variant;
    }
    pub async fn set_user_id(self: &Arc<Self>, user_id: i64) {
        *self.user_id.write().await = user_id;
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

        {
            ACTIVE_TASKS.lock().unwrap().push("Listener".to_string());
        }
        tokio::spawn(async move {
            while !*SHUTDOWN.read().await {
                if poll(Duration::from_millis(100)).unwrap() {
                    if let Some(msg) = read_half.next().await {
                        if *is_connected_out.lock().await == false {
                            log_message("Disconnected, not handeling incomming");
                            break;
                        }
                        Self::handle_message(
                            msg,
                            waiting_out.clone(),
                            writer_out.clone(),
                            is_connected_out.clone(),
                            sel_out.clone(),
                            variant.clone(),
                            sel_arc_out.clone(),
                        );
                    }
                }
            }
        });
        {
            ACTIVE_TASKS
                .lock()
                .unwrap()
                .retain(|t| !t.eq(&"Listener".to_string()));
        }
    }
    pub fn handle_message(
        msg: Result<Message, tungstenite::Error>,
        waiting: Arc<Mutex<HashMap<Uuid, Box<dyn Fn(CommunicationValue) + Send + Sync + 'static>>>>,
        writer: Arc<
            Mutex<
                Option<Box<dyn Sink<Message, Error = tungstenite::Error> + Send + Unpin + 'static>>,
            >,
        >,
        is_connected: Arc<Mutex<bool>>,
        sel: Arc<OmikronConnection>,
        variant: Arc<RwLock<ConnectionVariant>>,
        sel_arc: Arc<OmikronConnection>,
    ) {
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
                            let user_id: i64 = cv
                                .get_data(DataTypes::user_id)
                                .unwrap_or(&JsonValue::Null)
                                .as_i64()
                                .unwrap_or(0);
                            if user_id == 0 {
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

                            // Validate private key
                            if let Some(private_key_hash) = cv.get_data(DataTypes::private_key_hash)
                            {
                                log_message(format!("private_key_hash: {}", private_key_hash));
                                let is_valid = local_auth::is_private_key_valid(
                                    &user_id,
                                    &private_key_hash.to_string(),
                                );

                                if !is_valid {
                                    log_message("Invalid private key");
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

                            let response =
                                CommunicationValue::new(CommunicationType::identification_response)
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
                        let sender_id = &cv.get_sender();
                        let receiver_id = &cv.get_receiver();

                        chat_files::add_message(
                            cv.get_data(DataTypes::send_time)
                                .unwrap_or(&JsonValue::new_object())
                                .as_i64()
                                .unwrap_or(
                                    SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis() as i64,
                                ) as u128,
                            false,
                            *receiver_id,
                            *sender_id,
                            cv.get_data(DataTypes::content).unwrap().as_str().unwrap(),
                        );
                        let user_forward = CommunicationValue::new(CommunicationType::message_live)
                            .with_id(cv.get_id())
                            .with_receiver(*receiver_id)
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
                                JsonValue::Number(Number::from(cv.get_sender())),
                            );
                        Self::send_message_static(
                            &writer.clone(),
                            is_connected,
                            user_forward.to_json().to_string(),
                        )
                        .await;
                        return;
                    }

                    if cv.is_type(CommunicationType::message_send) {
                        let my_id = cv.get_sender();
                        let other_id = cv
                            .get_data(DataTypes::receiver_id)
                            .unwrap_or(&JsonValue::Null)
                            .as_i64()
                            .unwrap_or(0);
                        let now_ms = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u128;

                        chat_files::add_message(
                            now_ms,
                            true,
                            my_id,
                            other_id,
                            &*cv.get_data(DataTypes::content).unwrap().to_string(),
                        );

                        let ack = CommunicationValue::new(CommunicationType::success)
                            .with_id(cv.get_id())
                            .with_receiver(my_id);
                        Self::send_message_static(
                            &writer.clone(),
                            Arc::clone(&is_connected),
                            ack.to_json().to_string(),
                        )
                        .await;

                        let forward =
                            CommunicationValue::new(CommunicationType::message_other_iota)
                                .with_id(cv.get_id())
                                .with_receiver(other_id)
                                .add_data(
                                    DataTypes::receiver_id,
                                    JsonValue::Number(Number::from(other_id)),
                                )
                                .with_sender(my_id)
                                .add_data(
                                    DataTypes::send_time,
                                    JsonValue::String(now_ms.to_string()),
                                )
                                .add_data(
                                    DataTypes::sender_id,
                                    JsonValue::Number(Number::from(my_id)),
                                )
                                .add_data(
                                    DataTypes::content,
                                    JsonValue::String(
                                        cv.get_data(DataTypes::content).unwrap().to_string(),
                                    ),
                                );
                        Self::send_message_static(
                            &writer.clone(),
                            is_connected,
                            forward.to_json().to_string(),
                        )
                        .await;
                        return;
                    }

                    if cv.is_type(CommunicationType::messages_get) {
                        let my_id = cv.get_sender();
                        let partner_id = cv
                            .get_data(DataTypes::user_id)
                            .unwrap_or(&JsonValue::Null)
                            .as_i64()
                            .unwrap_or(0);
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
                        let messages = chat_files::get_messages(my_id, partner_id, offset, amount);
                        let resp = CommunicationValue::new(CommunicationType::messages_get)
                            .with_id(cv.get_id())
                            .with_receiver(my_id)
                            .add_data(DataTypes::messages, messages);

                        Self::send_message_static(
                            &writer.clone(),
                            is_connected,
                            resp.to_json().to_string(),
                        )
                        .await;
                        return;
                    }

                    if cv.is_type(CommunicationType::get_chats) {
                        let user_id = cv.get_sender();
                        let users = get_users(user_id);
                        let resp = CommunicationValue::new(CommunicationType::get_chats)
                            .with_id(cv.get_id())
                            .with_receiver(user_id)
                            .add_data(DataTypes::user_ids, users);
                        Self::send_message_static(
                            &writer.clone(),
                            is_connected,
                            resp.to_json().to_string(),
                        )
                        .await;
                        return;
                    }

                    if cv.is_type(CommunicationType::add_chat) {
                        let user_id = cv.get_sender();
                        let other_id = cv
                            .get_data(DataTypes::user_id)
                            .unwrap_or(&JsonValue::Null)
                            .as_i64()
                            .unwrap_or(0);
                        let mut contact =
                            get_user(user_id, other_id).unwrap_or(Contact::new(other_id));
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
                            is_connected,
                            resp.to_json().to_string(),
                        )
                        .await;
                        return;
                    }

                    if cv.is_type(CommunicationType::add_community) {
                        UserCommunityUtil::add_community(
                            cv.get_sender(),
                            cv.get_data(DataTypes::community_address)
                                .unwrap()
                                .to_string(),
                            cv.get_data(DataTypes::community_title).unwrap().to_string(),
                            cv.get_data(DataTypes::position).unwrap().to_string(),
                        );
                        let resp = CommunicationValue::new(CommunicationType::add_community)
                            .with_id(cv.get_id())
                            .with_receiver(cv.get_sender());
                        Self::send_message_static(
                            &writer.clone(),
                            is_connected,
                            resp.to_json().to_string(),
                        )
                        .await;
                        return;
                    }

                    if cv.is_type(CommunicationType::get_communities) {
                        let resp = CommunicationValue::new(CommunicationType::get_communities)
                            .with_id(cv.get_id())
                            .with_receiver(cv.get_sender())
                            .add_array(
                                DataTypes::communities,
                                UserCommunityUtil::get_communities(cv.get_sender()),
                            );
                        Self::send_message_static(
                            &writer.clone(),
                            is_connected,
                            resp.to_json().to_string(),
                        )
                        .await;
                        return;
                    }

                    if cv.is_type(CommunicationType::remove_community) {
                        UserCommunityUtil::remove_community(
                            cv.get_sender(),
                            cv.get_data(DataTypes::community_address)
                                .unwrap()
                                .to_string(),
                        ); // needs UserCommunityUtil
                        let resp = CommunicationValue::new(CommunicationType::remove_community)
                            .with_id(cv.get_id())
                            .with_receiver(cv.get_sender());
                        Self::send_message_static(
                            &writer.clone(),
                            is_connected,
                            resp.to_json().to_string(),
                        )
                        .await;
                        return;
                    }

                    if cv.is_type(CommunicationType::settings_save) {
                        let my_id = cv.get_sender();
                        let settings_name =
                            cv.get_data(DataTypes::settings_name).unwrap().to_string();
                        let settings_value = cv.get_data(DataTypes::payload).unwrap().to_string();

                        save_file(
                            &format!("users/{}/settings/", my_id),
                            &format!("{}.settings", settings_name),
                            &settings_value,
                        );

                        let response = CommunicationValue::new(CommunicationType::settings_save)
                            .with_receiver(my_id)
                            .with_id(cv.get_id());

                        Self::send_message_static(
                            &writer.clone(),
                            is_connected,
                            response.to_json().to_string(),
                        )
                        .await;
                        return;
                    }
                    if cv.is_type(CommunicationType::settings_load) {
                        let my_id = cv.get_sender();
                        let settings_name =
                            cv.get_data(DataTypes::settings_name).unwrap().to_string();
                        let settings_value_str = load_file(
                            &format!("users/{}/settings/", my_id),
                            &format!("{}.settings", settings_name),
                        );
                        let settings_value_json = JsonValue::from(settings_value_str);
                        let response = CommunicationValue::new(CommunicationType::settings_load)
                            .with_id(cv.get_id())
                            .with_receiver(my_id)
                            .add_data(DataTypes::payload, settings_value_json)
                            .add_data_str(DataTypes::settings_name, settings_name);

                        Self::send_message_static(
                            &writer.clone(),
                            is_connected,
                            response.to_json().to_string(),
                        )
                        .await;
                        return;
                    }
                    if cv.is_type(CommunicationType::settings_list) {
                        let my_id = cv.get_sender();
                        let settings = get_children(&format!("users/{}/settings/", my_id));
                        let mut settings_json = JsonValue::new_array();
                        for s in settings {
                            let s = s.replace(".settings", "");
                            if s.is_empty() {
                                continue;
                            }
                            let _ = settings_json.push(JsonValue::String(s));
                        }
                        let response = CommunicationValue::new(CommunicationType::settings_list)
                            .with_id(cv.get_id())
                            .with_receiver(my_id)
                            .add_data(DataTypes::settings, settings_json);

                        Self::send_message_static(
                            &writer.clone(),
                            is_connected,
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

    pub async fn send_message_static(
        writer: &Arc<
            Mutex<Option<Box<dyn Sink<Message, Error = tungstenite::Error> + Send + Unpin>>>,
        >,
        connected: Arc<Mutex<bool>>,
        msg: String,
    ) {
        let mut guard = writer.lock().await;
        if let Some(writer) = guard.as_mut() {
            match writer.send(Message::Text(Utf8Bytes::from(msg))).await {
                Ok(_) => match writer.flush().await {
                    Ok(_) => return,
                    Err(e) => {
                        log_message_format("send_message_failed", &[&e.to_string()]);
                        *connected.lock().await = false;
                    }
                },
                Err(e) => {
                    log_message_format("send_message_failed", &[&e.to_string()]);
                    *connected.lock().await = false;
                }
            }
        } else {
            log_message_format("send_message_failed", &["Immutable Writer"]);
            *connected.lock().await = false;
        }
    }
}
