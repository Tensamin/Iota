use base64::encode;
use futures::{StreamExt, TryFutureExt};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{
    Request as HttpRequest, Response as HttpResponse, StatusCode, body::Incoming,
    server::conn::http1, upgrade,
};
use hyper_util::rt::tokio::TokioIo;
use hyper_util::service::TowerToHyperService;
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use sha1::{Digest, Sha1};
use std::error::Error;
use std::io::ErrorKind;
use std::io::{self, BufReader};
use std::result::Result::Ok;
use std::sync::Arc;
use std::{future::Future, pin::Pin, time::Duration};
use tokio::net::TcpListener;
use tokio_tungstenite::WebSocketStream;
use tower::Service;

use crate::gui::log_panel::log_message;
use crate::server::socket::handle;
use crate::util::file_util::load_file_buf;
use tokio_rustls::TlsAcceptor;
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
                                log_message(format!("Handling WebSocket connection",));
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
                    log_message("No Sec-WebSocket-Key found in request headers");
                    // Handle error: No Sec-WebSocket-Key
                    let response = HttpResponse::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Full::new(Bytes::from("Missing Sec-WebSocket-Key")))
                        .unwrap();
                    Ok(response)
                }
            } else {
                let (status, body_text) = match path.as_str() {
                    "/" => (
                        StatusCode::OK,
                        "Server: Try connecting to WebSocket at ws://<host>:<port>/ws or check /status.",
                    ),
                    "/status" => (StatusCode::OK, "HTTP Server Status: Online"),
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

        Box::pin(fut.map_err(|err: color_eyre::eyre::ErrReport| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Error in request handling: {}", err),
            )
        }))
    }
}

pub async fn start(port: u16) -> bool {
    let tls_config = match load_tls_config() {
        Ok(config) => config,
        Err(e) => {
            log_message(format!("Failed to load TLS configuration: {}", e));
            log_message("Server stopped. Ensure 'certs/cert.pem' and 'certs/key.pem' exist.");
            return false;
        }
    };
    let acceptor = TlsAcceptor::from(tls_config);

    // Bind to the port
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await;
    if let Err(e) = listener {
        log_message(format!("Failed to bind to port {}: {:?}", port, e));
        return false;
    }
    let listener = listener.unwrap();
    log_message(format!(
        "Server listening for HTTP and WS on 0.0.0.0:{}",
        port
    ));

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                std::result::Result::Ok((stream, _addr)) => {
                    let service = HttpService;
                    let acceptor = acceptor.clone();

                    tokio::spawn(async move {
                        let tls_stream = match acceptor.accept(stream).await {
                            Ok(s) => s,
                            Err(e) => {
                                // Ignore non-TLS clients connecting to the TLS port
                                if e.kind() != io::ErrorKind::Interrupted {
                                    log_message(format!("TLS Handshake error: {:?}", e));
                                }
                                return;
                            }
                        };
                        let io = TokioIo::new(tls_stream);

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
fn calculate_accept_key(key: &str) -> String {
    let websocket_guid = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut sha1 = Sha1::new();
    sha1.update(key.as_bytes());
    sha1.update(websocket_guid.as_bytes());
    let result = sha1.finalize();
    encode(result) // Base64 encode the result
}
fn load_tls_config() -> Result<Arc<ServerConfig>, Box<dyn Error>> {
    // Load certificate file
    let mut cert_file = BufReader::new(load_file_buf("certs", "cert.pem")?);
    let cert_ders = rustls_pemfile::certs(&mut cert_file)
        .collect::<Result<Vec<CertificateDer>, io::Error>>()?;

    // PKCS8
    let mut key_file = BufReader::new(load_file_buf("certs", "cert.key")?);
    let mut key_ders = rustls_pemfile::pkcs8_private_keys(&mut key_file)
        .map(|r| r.map(Into::into)) // Explicit conversion
        .collect::<Result<Vec<PrivateKeyDer>, io::Error>>()?;

    if key_ders.is_empty() {
        // RSA
        key_file = BufReader::new(load_file_buf("certs", "cert.key")?);
        key_ders = rustls_pemfile::rsa_private_keys(&mut key_file)
            .map(|r| r.map(Into::into))
            .collect::<Result<Vec<PrivateKeyDer>, io::Error>>()?;
    }

    if key_ders.is_empty() {
        // EC
        key_file = BufReader::new(load_file_buf("certs", "cert.key")?);
        key_ders = rustls_pemfile::ec_private_keys(&mut key_file)
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

    Ok(Arc::new(config))
}
