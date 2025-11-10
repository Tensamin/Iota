use color_eyre::eyre::Ok;
use futures::{StreamExt, TryFutureExt};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{
    Request as HttpRequest, Response as HttpResponse, StatusCode, body::Incoming,
    server::conn::http1, upgrade,
};
use std::error::Error;
use std::io::{self};
use std::{future::Future, pin::Pin, time::Duration};
use tokio::net::TcpListener;
use tower::Service;
use warp::filters::log::log;

use crate::gui::log_panel::log_message;
use crate::langu::language_manager::format;
use crate::server::socket::handle;
use hyper_util::rt::tokio::TokioIo;
use hyper_util::service::TowerToHyperService;
use tokio_tungstenite::WebSocketStream;

#[derive(Clone)]
struct HttpService;

impl Service<HttpRequest<Incoming>> for HttpService {
    type Response = HttpResponse<Full<Bytes>>;
    type Error = io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(std::io::Result::Ok(()))
    }

    fn call(&mut self, req: HttpRequest<Incoming>) -> Self::Future {
        let path = req.uri().path().to_string();
        let headers = req.headers().clone();
        let upgrades = upgrade::on(req);

        let fut = async move {
            if path.starts_with("/ws")
                && headers
                    .get("connection")
                    .map(|v| v.to_str().unwrap_or("").contains("Upgrade"))
                    .unwrap_or(false)
                && headers.get("upgrade").map(|v| v.to_str().unwrap_or("")) == Some("websocket")
            {
                log_message("Attempting WebSocket upgrade on /ws");

                let response = HttpResponse::builder()
                    .status(StatusCode::SWITCHING_PROTOCOLS)
                    .header("Upgrade", "websocket")
                    .header("Connection", "Upgrade")
                    .body(Full::new(Bytes::from("")))
                    .unwrap();
                tokio::spawn(async move {
                    match upgrades.await {
                        std::result::Result::Ok(upgraded_stream) => {
                            let raw_stream = TokioIo::new(upgraded_stream);

                            let handshake_result = WebSocketStream::from_raw_socket(
                                raw_stream,
                                tungstenite::protocol::Role::Server,
                                None,
                            )
                            .await;
                            let (writer, reader) = handshake_result.split();
                            handle(path, writer, reader);
                        }
                        Err(e) => {
                            log_message(format!(
                                "WebSocket upgrade failed after response: {:?}",
                                e
                            ));
                        }
                    }
                });
                Ok(response)
            } else {
                let (status, body_text) = match path.as_str() {
                    "/" => (
                        StatusCode::OK,
                        "Barebones Server: Try connecting to WebSocket at ws://<host>:<port>/ws or check /status.",
                    ),
                    "/status" => (StatusCode::OK, "HTTP Server Status: Barebones Online"),
                    _ => (StatusCode::NOT_FOUND, "404 Not Found"),
                };
                let body = Full::new(Bytes::from(body_text.to_string()));
                let response = HttpResponse::builder()
                    .status(status)
                    .header(hyper::header::CONTENT_TYPE, "text/plain")
                    .body(body)
                    .unwrap();
                Ok(response)
            }
        };

        Box::pin(fut.map_err(|err| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Error in request handling: {}", err),
            )
        }))
    }
}

pub async fn start(port: u16) -> bool {
    // Bind to the port
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await;
    if let Err(e) = listener {
        log_message(format!("Failed to bind to port {}: {:?}", port, e));
        return false;
    }
    let listener = listener.unwrap();
    log_message(format!(
        "Barebones Server listening for HTTP and WS on 0.0.0.0:{}",
        port
    ));

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                std::result::Result::Ok((stream, _addr)) => {
                    let service = HttpService;
                    tokio::spawn(async move {
                        let io = TokioIo::new(stream);

                        if let Err(err) = http1::Builder::new()
                            .preserve_header_case(true)
                            .title_case_headers(true)
                            .serve_connection(io, TowerToHyperService::new(service))
                            .with_upgrades()
                            .await
                        {
                            if let Some(io_err) =
                                err.source().and_then(|e| e.downcast_ref::<io::Error>())
                            {
                                if io_err.kind() != io::ErrorKind::ConnectionReset
                                    && io_err.kind() != io::ErrorKind::BrokenPipe
                                {
                                    log_message(format!("Error serving connection: {:?}", err));
                                }
                            } else {
                                log_message(format!("Error serving connection: {:?}", err));
                            }
                        }
                    });
                }
                Err(e) => {
                    log_message(format!("Error accepting connection: {:?}", e));
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    });
    true
}
