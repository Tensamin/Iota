use crate::communities::{community_connection::CommunityConnection, community_manager};
use futures::StreamExt;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_hdr_async;
use tungstenite::handshake::server::{Request, Response};

pub async fn start(port: u16) -> bool {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await;
    if let Err(_) = listener {
        return false;
    }
    let listener = listener.unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut path: String = "/".to_string();
                let callback = |req: &Request, response: Response| {
                    path = format!("{}", &req.uri().path());
                    Ok(response)
                };
                let ws_stream = match accept_hdr_async(stream, callback).await {
                    Ok(ws) => ws,
                    Err(e) => {
                        return;
                    }
                };
                if path.starts_with("/community/") {
                    let community_id = path.split("/").nth(2).unwrap();
                    if let Some(community) = community_manager::get_community(community_id).await {
                        let community_conn: Arc<CommunityConnection> =
                            Arc::from(CommunityConnection::new(ws_stream, community));
                        loop {
                            let msg_result = {
                                let mut session_lock = community_conn.session.lock().await;
                                session_lock.next().await
                            };

                            match msg_result {
                                Some(Ok(msg)) => {
                                    if msg.is_text() {
                                        let text = msg.into_text().unwrap();
                                        community_conn.clone().handle_message(text).await;
                                    } else if msg.is_close() {
                                        community_conn.handle_close().await;
                                        return;
                                    }
                                }
                                Some(Err(e)) => {
                                    community_conn.handle_close().await;
                                    return;
                                }
                                None => {
                                    community_conn.handle_close().await;
                                    return;
                                }
                            }
                        }
                    }
                }
            });
        }
    });
    true
}
