use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use futures::stream::SplitStream;

pub fn handle(
    _path: String,
    _writer: SplitSink<WebSocket, Message>,
    _reader: SplitStream<WebSocket>,
) {
    tokio::spawn(async move {
        /*
        if path.starts_with("/ws/users/") {
            OmikronConnection::client(writer, reader).await;
        } else if path.starts_with("/ws/community/") {
            let community_id = path.split("/").nth(3).unwrap();
            log_message(format!("Community: {}", community_id));
            if let Some(community) = community_manager::get_community(community_id).await {
                log_message("Connected");
                let community_conn: Arc<CommunityConnection> =
                    Arc::from(CommunityConnection::new(writer, reader, community));
                loop {
                    if *SHUTDOWN.read().await {
                        break;
                    }
                    let msg_result = {
                        let mut session_lock = community_conn.receiver.write().await;
                        session_lock.next().await
                    };

                    match msg_result {
                        Some(Ok(msg)) => {
                            if msg.is_text() {
                                let text = msg.into_text().unwrap();
                                community_conn
                                    .clone()
                                    .handle_message(text.to_string())
                                    .await;
                            } else if msg.is_close() {
                                log_message(format!("Closing: {}", msg));
                                community_conn.handle_close().await;
                                return;
                            }
                        }
                        Some(Err(e)) => {
                            log_message(format!("Closing ERR: {}", e));
                            community_conn.handle_close().await;
                            return;
                        }
                        None => {
                            log_message("Closed Session me!");
                            community_conn.handle_close().await;
                            return;
                        }
                    }
                }
            }
        }
        */
    });
}
