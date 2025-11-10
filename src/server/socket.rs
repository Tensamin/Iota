use crate::communities::{community_connection::CommunityConnection, community_manager};

use async_tungstenite::accept_hdr_async;
use futures::StreamExt;
use futures::stream::SplitSink;
use futures::stream::SplitStream;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};
use tungstenite::{
    Message, Utf8Bytes,
    handshake::server::{Request, Response},
};

pub fn handle(
    path: String,
    writer: SplitSink<tokio_tungstenite::WebSocketStream<TokioIo<Upgraded>>, Message>,
    reader: SplitStream<tokio_tungstenite::WebSocketStream<TokioIo<Upgraded>>>,
) {
    tokio::spawn(async move {
        if path.starts_with("/ws/community/") {
            let community_id = path.split("/").nth(3).unwrap();
            if let Some(community) = community_manager::get_community(community_id).await {
                let community_conn: Arc<CommunityConnection> =
                    Arc::from(CommunityConnection::new(writer, reader, community));
                loop {
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
                                community_conn.handle_close().await;
                                return;
                            }
                        }
                        Some(Err(_)) => {
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

pub async fn start(port: u16) -> bool {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await;
    if let Err(_) = listener {
        return false;
    }
    let listener = listener.unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let mut path: String = "/".to_string();
            let callback = |req: &Request, response: Response| {
                path = format!("{}", &req.uri().path());
                Ok(response)
            };
            let ws_stream = match accept_hdr_async(stream.compat(), callback).await {
                Ok(ws) => ws,
                Err(_) => {
                    return;
                }
            };

            let (reader, writer) = ws_stream.split();
            //handle(path, reader, writer);
        }
    });
    true
}
