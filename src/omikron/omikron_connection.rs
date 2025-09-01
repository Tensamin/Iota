use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Utf8Bytes;
use std::collections::HashMap;
use std::io::Bytes;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};
use bytes;
use tokio_tungstenite::{
    connect_async,
    tungstenite::protocol::Message,
    MaybeTlsStream,
    WebSocketStream,
};
use uuid::Uuid;
use json::JsonValue;

use crate::{
    data::communication::CommunicationValue,
    data::communication::CommunicationType,
    data::communication::DataTypes
};

#[derive(Clone)]
pub struct OmikronConnection {
    writer: Arc<Mutex<Option<futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>>,
    waiting: Arc<Mutex<HashMap<Uuid, Box<dyn Fn(String) + Send>>>>,
}

impl OmikronConnection {
    pub fn new() -> Self {
        Self {
            writer: Arc::new(Mutex::new(None)),
            waiting: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn connect(&self) {
        loop {
            match connect_async("wss://tensamin.methanium.net/ws/iota/").await {
                Ok((ws_stream, _)) => {
                    println!("[Omikron] Connected to server");

                    // Split into writer + reader
                    let (write_half, read_half) = ws_stream.split();
                    *self.writer.lock().await = Some(write_half);

                    // Spawn listener with read_half
                    self.spawn_listener(read_half);
                    break;
                }
                Err(e) => {
                    println!(
                        "[Omikron] Connection failed: {}. Retrying in 2s...",
                        e
                    );
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    pub async fn close(&self) {
        let mut writer = self.writer.lock().await;
        if let Some(mut ws) = writer.take() {
            let _ = ws.close().await;
        }
        println!("[Omikron] Connection closed");
    }

    fn spawn_listener(
        &self,
        mut read_half: futures_util::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ) {
        let waiting = self.waiting.clone();

        tokio::spawn(async move {
            while let Some(msg) = read_half.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        println!("[Omikron] Message received: {}", text);

                        if text.contains("\"type\":\"pong\"") {
                            println!("[Omikron] Pong received");
                        }

                        // Example: trigger callback if message_id is present
                        if let Some(id_pos) = text.find("\"message_id\":\"") {
                            let s = &text[id_pos + 14..];
                            if let Some(end) = s.find('"') {
                                let mid = &s[..end];
                                if let Ok(uuid) = Uuid::parse_str(mid) {
                                    if let Some(callback) =
                                        waiting.lock().await.remove(&uuid)
                                    {
                                        callback(text.to_string());
                                    }
                                }
                            }
                        }
                    }
                    Ok(Message::Close(frame)) => {
                        println!("[Omikron] Closed: {:?}", frame);
                        break;
                    }
                    Err(e) => {
                        println!("[Omikron] Error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });
    }

    pub async fn send_message<'a>(&self, msg: String) {
        let mut guard = self.writer.lock().await;
        if let Some(writer) = guard.as_mut() {
            if let Err(e) = writer.send(Message::Text(Utf8Bytes::from(msg))).await {
                println!("[Omikron] Send failed: {}", e);
            }
        }
    }

    pub fn on_answer<F>(&self, message_id: Uuid, callback: F)
    where
        F: Fn(String) + Send + 'static,
    {
        tokio::spawn({
            let waiting = self.waiting.clone();
            async move {
                waiting.lock().await.insert(message_id, Box::new(callback));
            }
        });
    }
}

#[tokio::main]
async fn main() {
}
