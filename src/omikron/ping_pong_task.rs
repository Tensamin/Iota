use crate::APP_STATE;
use crate::data::communication::{CommunicationType, CommunicationValue, DataTypes};
use crate::gui::log_panel::AppState;
use crate::omikron::omikron_connection::OmikronConnection;
use color_eyre::owo_colors::OwoColorize;
use futures_util::{SinkExt, StreamExt};
use std::arch::x86_64::_SIDD_MASKED_NEGATIVE_POLARITY;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant, sleep};
use uuid::Uuid;

#[derive(Clone)]
pub struct PingPongTask {
    pub parent: Arc<OmikronConnection>,
    pub message_send_times: Arc<Mutex<HashMap<Uuid, Instant>>>,
    pub no_ping_in: Arc<Mutex<i32>>,
    pub last_ping: Arc<Mutex<Option<u64>>>,
}
fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn check_omikron_connection_send_sync() {
    assert_send_sync::<OmikronConnection>();
}

#[test]
fn check_pingpong_task_send_sync() {
    assert_send_sync::<PingPongTask>();
}
impl PingPongTask {
    pub fn new(parent: Arc<OmikronConnection>) -> Self {
        let message_send_times = Arc::new(Mutex::new(HashMap::new()));
        let no_ping_in = Arc::new(Mutex::new(-1));
        let last_ping = Arc::new(Mutex::new(None));

        let task = PingPongTask {
            parent: parent.clone(),
            message_send_times: message_send_times.clone(),
            no_ping_in: no_ping_in.clone(),
            last_ping: last_ping.clone(),
        };

        task
    }

    pub fn send_ping(&self) {
        let sel = self.clone();
        tokio::spawn(async move {
            let uuid = Uuid::new_v4();
            let send_time = Instant::now();
            let mut message_send_times = sel.message_send_times.lock().await;
            message_send_times.insert(uuid, send_time);

            let no_ping_in_val = {
                let no_ping_in = sel.no_ping_in.lock().await;
                *no_ping_in
            };

            if no_ping_in_val != -1 {
                sel.handle_slow_connection(no_ping_in_val).await;
            } else {
                sel.parent.send_ping_message(uuid).await;
            }
        });
    }

    pub async fn handle_slow_connection(&self, no_ping_in: i32) {
        if no_ping_in > 8 {
            self.parent.reconnect().await;
            self.reconnect().await;
        }
    }

    pub async fn reconnect(&self) {
        let mut no_ping_in = self.no_ping_in.lock().await;
        *no_ping_in = -1;
    }

    pub fn handle_pong(&self, cv: &CommunicationValue, log: bool) {
        let sel = self.clone();
        let cv = cv.clone();
        tokio::spawn(async move {
            let send_time = {
                let message_send_times = sel.message_send_times.lock().await;
                message_send_times.get(&cv.get_id()).cloned()
            };

            if let Some(send_time) = send_time {
                let receive_time = Instant::now();
                let ping = receive_time.duration_since(send_time).as_millis() as u64;

                let mut last_ping = sel.last_ping.lock().await;
                *last_ping = Some(ping);
                let mut no_ping_in = sel.no_ping_in.lock().await;
                *no_ping_in = -1;
                let mut message_send_times = sel.message_send_times.lock().await;
                message_send_times.remove(&cv.get_id());

                if log {
                    APP_STATE.lock().unwrap().push_log("12.0".to_string());
                    APP_STATE.lock().unwrap().push_ping_val(12.0);
                }
            }
        });
    }

    pub async fn cancel(&self) {
        let mut no_ping_in = self.no_ping_in.lock().await;
        *no_ping_in = -1;
    }
}
