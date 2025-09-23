use crate::data::communication::{CommunicationType, CommunicationValue, DataTypes};
use crate::omikron::omikron_connection::OmikronConnection;
use color_eyre::owo_colors::OwoColorize;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant, sleep};
use uuid::Uuid;

#[derive(Clone)]
pub struct PingPongTask {
    pub parent: Arc<OmikronConnection>, // assuming OmikronConnection is your connection type
    pub message_send_times: Arc<Mutex<HashMap<Uuid, Instant>>>,
    pub no_ping_in: Arc<Mutex<i32>>,
    pub last_ping: Arc<Mutex<Option<u64>>>,
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

        // Spawn the periodic ping task
        tokio::spawn({
            let task = task.clone(); // Clone the task (Arc) so that it lives long enough for the async task
            async move {
                task.run_ping_loop().await;
            }
        });

        task
    }

    pub async fn run_ping_loop(&self) {
        loop {
            sleep(Duration::from_secs(5)).await;
            self.send_ping().await;
        }
    }

    pub async fn send_ping(&self) {
        let uuid = Uuid::new_v4();
        let send_time = Instant::now();

        {
            let mut message_send_times = self.message_send_times.lock().await;
            message_send_times.insert(uuid, send_time);
        }

        let no_ping_in = {
            let no_ping_in = self.no_ping_in.lock().await;
            *no_ping_in
        };

        if no_ping_in != -1 {
            // Connection slow or disconnected
            self.handle_slow_connection(no_ping_in).await;
        } else {
            // Connection is fine
            self.parent.send_ping_message(uuid).await;
        }
    }

    pub async fn handle_slow_connection(&self, no_ping_in: i32) {
        if no_ping_in > 8 {
            // Attempt reconnection if ping times out
            self.parent.reconnect().await;
            self.reconnect().await;
        }
    }

    pub async fn reconnect(&self) {
        let mut no_ping_in = self.no_ping_in.lock().await;
        *no_ping_in = -1; // Reset slow count
    }

    pub async fn handle_pong(&self, cv: &CommunicationValue) {
        let send_time = {
            let message_send_times = self.message_send_times.lock().await;
            message_send_times.get(&cv.get_id()).cloned()
        };

        if let Some(send_time) = send_time {
            let receive_time = Instant::now();
            let ping = receive_time.duration_since(send_time).as_millis() as u64;

            {
                let mut last_ping = self.last_ping.lock().await;
                *last_ping = Some(ping);
            }

            {
                let mut no_ping_in = self.no_ping_in.lock().await;
                *no_ping_in = -1;
            }
        }
    }

    pub async fn cancel(&self) {
        // Cancel or stop the task
        let mut no_ping_in = self.no_ping_in.lock().await;
        *no_ping_in = -1; // Reset the counter

        // Example of stopping the ping task gracefully
        // If using a task join handle or similar
    }
}
