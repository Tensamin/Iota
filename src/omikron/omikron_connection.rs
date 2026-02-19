use crate::users::contact::Contact;
use crate::users::user_community_util::UserCommunityUtil;
use crate::util::chat_files::{MessageState, change_message_state};
use crate::util::chats_util::{get_user, mod_user};
use crate::util::crypto_util::{DataFormat, SecurePayload};
use crate::util::file_util::{get_children, load_file, save_file};
use crate::util::{chat_files, chats_util};
use crate::{ACTIVE_TASKS, SHUTDOWN, log, log_cv_in, log_cv_out, log_t};
use crate::{
    data::communication::{CommunicationType, CommunicationValue, DataTypes},
    util::{config_util::CONFIG, crypto_helper},
};
use dashmap::DashMap;
use futures::Stream;
use futures_util::sink::Sink;
use futures_util::{SinkExt, StreamExt};
use json::JsonValue;
use json::number::Number;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::time::{Duration, Instant, sleep};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tungstenite::Utf8Bytes;
use uuid::Uuid;
use warp::filters::log::log;

pub static OMIKRON_CONNECTION: LazyLock<Arc<RwLock<Option<Arc<OmikronConnection>>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

#[derive(Clone)]
pub struct OmikronConnection {
    pub(crate) writer:
        Arc<Mutex<Option<Box<dyn Sink<Message, Error = tungstenite::Error> + Send + Unpin>>>>,
    waiting: Arc<DashMap<Uuid, Box<dyn Fn(CommunicationValue) + Send + Sync>>>,
    pub last_ping: Arc<Mutex<i64>>,
    pub message_send_times: Arc<Mutex<HashMap<Uuid, Instant>>>,
    pub is_connected: Arc<Mutex<bool>>,
}

impl OmikronConnection {
    pub fn new() -> Self {
        Self {
            writer: Arc::new(Mutex::new(None)),
            waiting: Arc::new(DashMap::new()),
            last_ping: Arc::new(Mutex::new(-1)),
            message_send_times: Arc::new(Mutex::new(HashMap::new())),
            is_connected: Arc::new(Mutex::new(false)),
        }
    }
    pub async fn is_connected(&self) -> bool {
        *self.is_connected.lock().await
    }

    pub async fn connect(self: &Arc<Self>) {
        if self.is_connected().await {
            return;
        }

        let conf = CONFIG.read().await;
        let iota_id = conf.get_iota_id();
        let public_key = conf.get_public_key();
        let private_key = conf.get_private_key();
        drop(conf);

        if iota_id == 0 || public_key.is_none() || private_key.is_none() {
            log_t!("iota_register_new");
            let key_pair = crypto_helper::generate_keypair();
            let public_key_base64 = crypto_helper::public_key_to_base64(&key_pair.public);
            let private_key_base64 = crypto_helper::secret_key_to_base64(&key_pair.secret);

            let mut conf_write = CONFIG.write().await;
            conf_write.change("public_key", JsonValue::String(public_key_base64.clone()));
            conf_write.change("private_key", JsonValue::String(private_key_base64));
            conf_write.update();
            drop(conf_write);

            if self.connect_internal().await {
                match self
                    .clone()
                    .await_response(
                        &CommunicationValue::new(CommunicationType::register_iota)
                            .add_data(DataTypes::public_key, JsonValue::String(public_key_base64)),
                        Some(Duration::from_secs(20)),
                    )
                    .await
                {
                    Ok(response_cv) => {
                        let iota_json = response_cv
                            .get_data(DataTypes::register_id)
                            .unwrap_or(&JsonValue::Null);
                        let iota_id = iota_json.as_i64().unwrap_or(0);

                        let mut conf_write = CONFIG.write().await;
                        conf_write.change("iota_id", iota_json.clone());
                        conf_write.update();
                        drop(conf_write);
                        log!("Registered with Iota-ID: {}", iota_id);
                    }
                    Err(timeout) => {
                        log!("{}", timeout);
                    }
                }
            }
        } else {
            if self.connect_internal().await {
                self.send_message(
                    &CommunicationValue::new(CommunicationType::identification).add_data(
                        DataTypes::iota_id,
                        JsonValue::Number(json::number::Number::from(iota_id)),
                    ),
                )
                .await;
            }
        }
    }

    async fn connect_internal(self: &Arc<Self>) -> bool {
        if self.is_connected().await {
            return true;
        }
        log_t!("omikron_connecting");

        let conf = CONFIG.read().await;
        let addr = conf
            .get("omikron_addr")
            .as_str()
            .unwrap_or("wss://app.tensamin.net/ws/iota/");
        let stream_res = connect_async(addr).await;
        if let Err(e) = stream_res {
            log!("con error {}", e.to_string());
            return false;
        }
        let (stream, _) = stream_res.unwrap();
        log_t!("omikron_connection_success");

        let (write_half, read_half) = stream.split();

        *self.writer.lock().await = Some(Box::new(write_half));
        let boxed_reader: Box<
            dyn Stream<Item = Result<Message, tungstenite::Error>> + Send + Unpin,
        > = Box::new(read_half);
        self.spawn_listener(boxed_reader).await;

        let mut is_connected = self.is_connected.lock().await;
        *is_connected = true;
        drop(is_connected);

        let sel_arc_clone = self.clone();
        tokio::spawn(async move {
            loop {
                if *SHUTDOWN.read().await {
                    break;
                }
                if !sel_arc_clone.is_connected().await {
                    break;
                }
                sel_arc_clone.send_ping().await;
                sleep(Duration::from_secs(10)).await;
            }
        });

        true
    }

    pub async fn send_message(&self, cv: &CommunicationValue) {
        if !cv.is_type(CommunicationType::ping) {
            log_cv_out!(cv);
        }
        Self::send_message_static(
            &self.writer,
            Arc::clone(&self.is_connected),
            cv.to_json().to_string(),
        )
        .await;
    }

    async fn spawn_listener(
        self: &Arc<Self>,
        mut read_half: Box<dyn Stream<Item = Result<Message, tungstenite::Error>> + Send + Unpin>,
    ) {
        let waiting_out = self.waiting.clone();
        let writer_out = self.writer.clone();
        let is_connected_out = self.is_connected.clone();
        let sel_out = self.clone();

        {
            ACTIVE_TASKS.insert("Omikron Listener".to_string());
        }
        tokio::spawn(async move {
            while let Some(msg) = read_half.next().await {
                if *SHUTDOWN.read().await {
                    break;
                }
                sel_out
                    .clone()
                    .handle_message(
                        msg,
                        waiting_out.clone(),
                        writer_out.clone(),
                        is_connected_out.clone(),
                    )
                    .await;
            }
            *is_connected_out.lock().await = false;
            log!("Connection closed.");
            {
                ACTIVE_TASKS.remove("Omikron Listener");
            }
        });
    }
    pub async fn handle_message(
        self: Arc<Self>,
        msg: Result<Message, tungstenite::Error>,
        waiting: Arc<DashMap<Uuid, Box<dyn Fn(CommunicationValue) + Send + Sync + 'static>>>,
        writer: Arc<
            Mutex<
                Option<Box<dyn Sink<Message, Error = tungstenite::Error> + Send + Unpin + 'static>>,
            >,
        >,
        is_connected: Arc<Mutex<bool>>,
    ) {
        match msg {
            Ok(Message::Close(Some(frame))) => {
                log!("[Omikron] Closed: {:?}", frame);
                *is_connected.lock().await = false;
                return;
            }
            Ok(Message::Text(text)) => {
                let cv = CommunicationValue::from_json(&text);
                if let Some((_, y)) = waiting.remove(&cv.get_id()) {
                    y(cv);
                    return;
                }
                if cv.is_type(CommunicationType::pong) {
                    self.handle_pong(&cv, true).await;
                    return;
                }
                log_cv_in!(&cv);
                if cv.is_type(CommunicationType::challenge) {
                    let conf = CONFIG.read().await;
                    let private_key = conf.get_private_key().unwrap();
                    drop(conf);

                    let omikron_public_key = cv
                        .get_data(DataTypes::public_key)
                        .unwrap()
                        .as_str()
                        .unwrap();
                    let encrypted_challenge =
                        cv.get_data(DataTypes::challenge).unwrap().as_str().unwrap();

                    let solved_challenge = {
                        if let Ok(decrypted) = SecurePayload::new(
                            encrypted_challenge,
                            DataFormat::Base64,
                            crypto_helper::load_secret_key(&private_key).unwrap(),
                        ) {
                            if let Ok(decrypted) = decrypted.decrypt_x448(
                                crypto_helper::load_public_key(omikron_public_key).unwrap(),
                            ) {
                                Some(decrypted)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };

                    if let Some(decrypted) = solved_challenge {
                        let response =
                            CommunicationValue::new(CommunicationType::challenge_response)
                                .with_id(cv.get_id())
                                .add_data(
                                    DataTypes::challenge,
                                    JsonValue::String(decrypted.export(DataFormat::Base64)),
                                );

                        self.send_message(&response).await;
                    } else {
                        log!("Failed to decrypt challenge");
                    }

                    return;
                }
                if cv.is_type(CommunicationType::success) {
                    let iota_id = cv
                        .get_data(DataTypes::iota_id)
                        .unwrap_or(&JsonValue::Null)
                        .as_i64()
                        .unwrap_or(0);
                    if iota_id != 0 {
                        let mut conf = CONFIG.write().await;
                        conf.change("iota_id", JsonValue::Number(iota_id.into()));
                        conf.update();
                        log!("Iota registered with ID: {}", iota_id);

                        let login_message =
                            CommunicationValue::new(CommunicationType::identification).add_data(
                                DataTypes::iota_id,
                                JsonValue::Number(json::number::Number::from(iota_id)),
                            );

                        let self_clone = self.clone();
                        tokio::spawn(async move {
                            self_clone.send_message(&login_message).await;
                        });
                    } else {
                        log("Iota registration failed.");
                    }
                    return;
                }
                if cv.is_type(CommunicationType::identification_response) {
                    if let Some(accepted) = cv.get_data(DataTypes::accepted) {
                        log!("Omikron connected: {}", accepted.to_string());
                    }
                    return;
                }

                // ************************************************ //
                // Direct messages                                  //
                // ************************************************ //
                if cv.is_type(CommunicationType::message_state) {
                    let sender_id = &cv.get_sender();
                    let receiver_id = &cv.get_receiver();

                    let _ = chat_files::change_message_state(
                        cv.get_data(DataTypes::send_time)
                            .unwrap_or(&JsonValue::new_object())
                            .as_i64()
                            .unwrap_or(0) as i64,
                        *receiver_id,
                        *sender_id,
                        MessageState::from_str(
                            cv.get_data(DataTypes::message_state)
                                .unwrap_or(&JsonValue::Null)
                                .as_str()
                                .unwrap_or(""),
                        ),
                    );
                }
                if cv.is_type(CommunicationType::message_other_iota) {
                    let sender_id = &cv.get_sender();
                    let receiver_id = &cv.get_receiver();
                    let timestamp = cv
                        .get_data(DataTypes::send_time)
                        .unwrap_or(&JsonValue::new_object())
                        .as_i64()
                        .unwrap_or(
                            SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as i64,
                        );
                    chat_files::add_message(
                        timestamp as u128,
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
                    let user_resp = self
                        .clone()
                        .await_response(&user_forward, Some(Duration::from_secs(10)))
                        .await;

                    if let Ok(user_resp) = user_resp {
                        let ms = MessageState::from_str(
                            user_resp
                                .get_data(DataTypes::message_state)
                                .unwrap_or(&JsonValue::Null)
                                .as_str()
                                .unwrap_or(""),
                        )
                        .upgrade(MessageState::Received);
                        let _ =
                            change_message_state(timestamp, *receiver_id, *sender_id, ms.clone());
                        self.send_message(
                            &CommunicationValue::new(CommunicationType::message_state)
                                .with_id(cv.get_id())
                                .with_receiver(*sender_id)
                                .with_sender(*receiver_id)
                                .add_data(
                                    DataTypes::send_time,
                                    cv.get_data(DataTypes::send_time).unwrap().clone(),
                                )
                                .add_data(DataTypes::message_state, JsonValue::from(ms.as_str())),
                        )
                        .await;
                    } else {
                        self.send_message(
                            &CommunicationValue::new(CommunicationType::message_state)
                                .with_id(cv.get_id())
                                .with_receiver(*sender_id)
                                .with_sender(*receiver_id)
                                .add_data(
                                    DataTypes::send_time,
                                    cv.get_data(DataTypes::send_time).unwrap().clone(),
                                )
                                .add_data(
                                    DataTypes::message_state,
                                    JsonValue::from(MessageState::Sent.as_str()),
                                ),
                        )
                        .await;
                    }
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

                    let forward = CommunicationValue::new(CommunicationType::message_other_iota)
                        .with_id(cv.get_id())
                        .with_receiver(other_id)
                        .add_data(
                            DataTypes::receiver_id,
                            JsonValue::Number(Number::from(other_id)),
                        )
                        .with_sender(my_id)
                        .add_data(DataTypes::send_time, JsonValue::String(now_ms.to_string()))
                        .add_data(DataTypes::sender_id, JsonValue::Number(Number::from(my_id)))
                        .add_data(
                            DataTypes::content,
                            JsonValue::String(cv.get_data(DataTypes::content).unwrap().to_string()),
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
                    let users = chats_util::get_users(user_id);
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

                if cv.is_type(CommunicationType::add_conversation) {
                    let user_id = cv.get_sender();
                    let other_id = cv
                        .get_data(DataTypes::chat_partner_id)
                        .unwrap_or(&JsonValue::Null)
                        .as_i64()
                        .unwrap_or(0);
                    let mut contact = get_user(user_id, other_id).unwrap_or(Contact::new(other_id));
                    contact.set_last_message_at(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as i64,
                    );
                    mod_user(user_id, &contact);
                    let resp = CommunicationValue::new(CommunicationType::add_conversation)
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
                    let settings_name = cv.get_data(DataTypes::settings_name).unwrap().to_string();
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
                    let settings_name = cv.get_data(DataTypes::settings_name).unwrap().to_string();
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
                log!("Omikron] Error: {}", e);
                *is_connected.lock().await = false;
                return;
            }
            _ => {}
        }
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
            if let Err(e) = writer.send(Message::Text(Utf8Bytes::from(msg))).await {
                log_t!("send_message_failed", e.to_string());
                *connected.lock().await = false;
                return;
            }

            if let Err(e) = writer.flush().await {
                log_t!("send_message_failed", e.to_string());
                *connected.lock().await = false;
            }
        } else {
            log_t!("send_message_failed", "Writer not initialized".to_string());
            *connected.lock().await = false;
        }
    }

    pub async fn await_response(
        &self,
        cv: &CommunicationValue,
        timeout_duration: Option<Duration>,
    ) -> Result<CommunicationValue, String> {
        let (tx, mut rx) = mpsc::channel(1);
        let msg_id = cv.get_id();

        let task_tx = tx.clone();
        self.waiting.insert(
            msg_id,
            Box::new(move |response_cv| {
                let inner_tx = task_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = inner_tx.send(response_cv).await {
                        log_t!("Failed to send response back to awaiter: {}", e.to_string());
                    }
                });
            }),
        );

        self.send_message(&cv).await;

        let timeout = timeout_duration.unwrap_or(Duration::from_secs(10));

        match tokio::time::timeout(timeout, rx.recv()).await {
            Ok(Some(response_cv)) => Ok(response_cv),
            Ok(_) => Err("Failed to receive response, channel was closed.".to_string()),
            Err(_) => {
                self.waiting.remove(&msg_id);
                Err(format!(
                    "Request timed out after {} seconds.",
                    timeout.as_secs()
                ))
            }
        }
    }
}
