use crate::gui::log_panel::log_message;
<<<<<<< HEAD
use crate::server::api::{self, api_router};
=======
>>>>>>> f0d04474165a8c397b527eedd59263390462af95
use crate::server::socket::handle;
use crate::server::{api, web_path_parser};
use crate::util::file_util::load_file_buf;
use crate::{ACTIVE_TASKS, SHUTDOWN};

use axum::{
    Router,
    extract::{
        ConnectInfo, Path,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{any, get},
};
use axum_server::tls_rustls::RustlsConfig;
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::{
    error::Error,
    io::{self, BufReader, ErrorKind},
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};
use tokio::net::TcpListener;
<<<<<<< HEAD
use tower::ServiceBuilder;

fn build_router(ssl: bool) -> Router {
    Router::new()
        .route("/ws/*path", get(ws_handler))
        .nest("/api/*path", api_router())
        .route("/*path", any(static_handler))
        .layer(ServiceBuilder::new())
        .with_state(ssl)
=======
use tokio::sync::broadcast;
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::WebSocketStream;
use tower::Service;
#[derive(Clone)]
struct HttpService {
    peer_addr: SocketAddr,
    ssl: bool,
>>>>>>> f0d04474165a8c397b527eedd59263390462af95
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(path): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    log_message(format!("WS connection from {}", addr));

<<<<<<< HEAD
    ws.on_upgrade(move |socket| async move {
        handle_ws(socket, path).await;
    })
=======
    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(std::io::Result::Ok(()))
    }

    fn call(&mut self, req: HttpRequest<Incoming>) -> Self::Future {
        let peer_ip = self.peer_addr.ip();

        let is_acceptable = is_local_network(peer_ip) || self.ssl;

        let (parts, body) = req.into_parts();

        let method = parts.method.clone();
        let path = parts.uri.path().to_string();
        let headers = parts.headers.clone();

        let fut = async move {
            let is_websocket_upgrade = path.starts_with("/ws")
                && method == Method::GET
                && headers
                    .get("connection")
                    .map(|v| v.to_str().unwrap_or("").contains("Upgrade"))
                    .unwrap_or(false)
                && headers.get("upgrade").map(|v| v.to_str().unwrap_or("")) == Some("websocket");

            if is_websocket_upgrade {
                log_message("Attempting WebSocket upgrade on /ws");

                if let Some(sec_websocket_key) = headers.get("sec-websocket-key") {
                    let sec_websocket_key = sec_websocket_key.to_str().unwrap_or("").to_string();
                    let sec_websocket_accept = calculate_accept_key(&sec_websocket_key);

                    let response = HttpResponse::builder()
                        .status(StatusCode::SWITCHING_PROTOCOLS)
                        .header("Upgrade", "websocket")
                        .header("Connection", "Upgrade")
                        .header("Sec-WebSocket-Accept", sec_websocket_accept)
                        .body(Full::new(Bytes::from("")))
                        .unwrap();
                    let req_for_upgrade = HttpRequest::from_parts(parts, body);
                    let upgrades = upgrade::on(req_for_upgrade);

                    match upgrades.await {
                        std::result::Result::Ok(upgraded_stream) => {
                            let raw_stream = TokioIo::new(upgraded_stream);

                            let handshake_result = WebSocketStream::from_raw_socket(
                                raw_stream,
                                tungstenite::protocol::Role::Server,
                                None,
                            )
                            .await;
                            log_message(format!("Handling WebSocket connection",));
                            let (writer, reader) = handshake_result.split();
                            handle(path.clone(), writer, reader);
                        }
                        Err(e) => {
                            log_message(format!(
                                "WebSocket upgrade failed after response: {:?}",
                                e
                            ));
                        }
                    }
                    Ok(response)
                } else {
                    log_message("No Sec-WebSocket-Key found in request headers");
                    let response = HttpResponse::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Full::new(Bytes::from("Missing Sec-WebSocket-Key")))
                        .unwrap();
                    Ok(response)
                }
            } else if path.starts_with("/api") {
                let whole_body = match body.collect().await {
                    Ok(collected) => collected,
                    Err(e) => {
                        log_message(format!("Error collecting body: {}", e));
                        return Ok(HttpResponse::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Full::new(Bytes::from(format!(
                                "Failed to read body: {}",
                                e
                            ))))
                            .unwrap());
                    }
                };
                let bytes = whole_body.to_bytes();

                let body_string: Option<String> = match String::from_utf8(bytes.to_vec()) {
                    Ok(s) => Some(s),
                    Err(_) => None,
                };

                Ok(api::handle(&path, &is_acceptable, headers.clone(), body_string).await)
            } else {
                let whole_body = match body.collect().await {
                    Ok(collected) => collected,
                    Err(e) => {
                        log_message(format!("Error collecting body: {}", e));
                        return Ok(HttpResponse::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Full::new(Bytes::from(format!(
                                "Failed to read body: {}",
                                e
                            ))))
                            .unwrap());
                    }
                };
                let bytes = whole_body.to_bytes();

                let body_string: Option<String> = match String::from_utf8(bytes.to_vec()) {
                    Ok(s) => Some(s),
                    Err(_) => None,
                };

                Ok(web_path_parser::handle(&path, headers, body_string).await)
            }
        };

        Box::pin(fut.map_err(|err: color_eyre::eyre::ErrReport| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Error in request handling: {}", err),
            )
        }))
    }
>>>>>>> f0d04474165a8c397b527eedd59263390462af95
}

async fn handle_ws(socket: WebSocket, path: String) {
    let (mut sender, mut receiver) = socket.split();
    handle(path, sender, receiver);
}

async fn static_handler(Path(path): Path<String>) -> impl IntoResponse {
    let mut parts: Vec<&str> = path.split('/').collect();
    let name = parts.pop().unwrap_or("index.html");

    let name = if name.is_empty() {
        "index.html"
    } else if name.contains('.') {
        name
    } else {
        &format!("{}.html", name)
    };

    let content = load_file_vec(&format!("web{}/", parts.join("/")), name);

    if content.is_empty() {
        return (StatusCode::NOT_FOUND, load_file_vec("web", "404.html"));
    }

    let mime = match name.split('.').last().unwrap_or("") {
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    };

    (StatusCode::OK, content)
}

pub async fn start(port: u16) -> bool {
    match load_tls_config() {
        Ok(Some(tls)) => run_tls_server(port, tls).await,
        Ok(_) => run_http_server(port).await,
        Err(e) => {
            log_message(format!("TLS config error: {}", e));
            false
        }
    }
}

async fn run_http_server(port: u16) -> bool {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let router = build_router(false);

    log_message(format!("HTTP Server running on {}", addr));

    ACTIVE_TASKS.lock().unwrap().push("WebServer".into());

    tokio::spawn(async move {
        let listener = TcpListener::bind(addr).await.unwrap();

        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(wait_for_shutdown())
        .await
        .unwrap();

        ACTIVE_TASKS.lock().unwrap().retain(|t| t != "WebServer");
        log_message("HTTP Server shutdown complete.");
    });

    true
}

async fn run_tls_server(port: u16, tls: Arc<ServerConfig>) -> bool {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let router = build_router(true);

    let tls_config = RustlsConfig::from_config(tls);

    log_message(format!("HTTPS (HTTP/2) Server running on {}", addr));

    ACTIVE_TASKS.lock().unwrap().push("WebServer".into());

    tokio::spawn(async move {
        axum_server::bind_rustls(addr, tls_config)
            .serve(router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap();

        ACTIVE_TASKS.lock().unwrap().retain(|t| t != "WebServer");
        log_message("HTTPS Server shutdown complete.");
    });

    true
}

async fn wait_for_shutdown() {
    loop {
        if *SHUTDOWN.read().await {
            log_message("Shutdown signal received.");
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
fn load_tls_config() -> Result<Option<Arc<ServerConfig>>, Box<dyn Error>> {
    let cert_file_res = load_file_buf("certs", "cert.pem");
    let key_file_res = load_file_buf("certs", "cert.key");

    let cert_file_buf = match cert_file_res {
        Ok(b) => b,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            log_message("TLS certificate 'certs/cert.pem' not found.");
            return Ok(None);
        }
        Err(e) => return Err(e.into()), // Other IO error
    };

    let key_file_buf = match key_file_res {
        Ok(b) => b,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            log_message("TLS key 'certs/cert.key' not found.");
            return Ok(None);
        }
        Err(e) => return Err(e.into()), // Other IO error
    };

    // Continue with configuration if both files were found
    let mut cert_reader = BufReader::new(cert_file_buf);
    let cert_ders = rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<Vec<CertificateDer>, io::Error>>()?;

    // PKCS8
    let mut key_reader = BufReader::new(key_file_buf);
    let mut key_ders = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
        .map(|r| r.map(Into::into)) // Explicit conversion
        .collect::<Result<Vec<PrivateKeyDer>, io::Error>>()?;

    if key_ders.is_empty() {
        // RSA
        key_reader = BufReader::new(load_file_buf("certs", "cert.key")?); // Re-read key file
        key_ders = rustls_pemfile::rsa_private_keys(&mut key_reader)
            .map(|r| r.map(Into::into))
            .collect::<Result<Vec<PrivateKeyDer>, io::Error>>()?;
    }

    if key_ders.is_empty() {
        // EC
        key_reader = BufReader::new(load_file_buf("certs", "cert.key")?); // Re-read key file
        key_ders = rustls_pemfile::ec_private_keys(&mut key_reader)
            .map(|r| r.map(Into::into))
            .collect::<Result<Vec<PrivateKeyDer>, io::Error>>()?;
    }

    if key_ders.is_empty() {
        return Err("No private keys found in key file. (Tried PKCS8, RSA, and EC)".into());
    }

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_ders, key_ders.remove(0))
        .map_err(|e| io::Error::new(ErrorKind::Other, e.to_string()))?;

    Ok(Some(Arc::new(config)))
}
pub fn is_local_network(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            o[0] == 10
                || (o[0] == 172 && (16..=31).contains(&o[1]))
                || (o[0] == 192 && o[1] == 168)
                || o[0] == 127
                || (o[0] == 169 && o[1] == 254)
        }
        IpAddr::V6(v6) => {
            let s = v6.segments();
            (s[0] & 0xfe00) == 0xfc00 || (s[0] & 0xffc0) == 0xfe80 || v6.is_loopback()
        }
    }
}
