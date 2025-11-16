use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use futures::{StreamExt, TryFutureExt};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{
    Request as HttpRequest, Response as HttpResponse, StatusCode, body::Incoming,
    server::conn::http1, upgrade,
};
use hyper_util::rt::tokio::TokioIo;
use hyper_util::service::TowerToHyperService;
// FIX: Add necessary rustls imports for builder in minimal-feature environment
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use sha1::{Digest, Sha1};
use std::error::Error;
use std::io::ErrorKind;
use std::io::{self, BufReader};
use std::net::{IpAddr, SocketAddr};
use std::result::Result::Ok;
use std::sync::Arc;
use std::{future::Future, pin::Pin, time::Duration};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::WebSocketStream;
use tower::Service;

use crate::gui::log_panel::log_message;
use crate::server::socket::handle;
use crate::util::file_util::load_file_buf;
use tokio_rustls::TlsAcceptor;

#[derive(Clone)]
struct HttpService {
    peer_addr: SocketAddr,
}

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

        let peer_ip = self.peer_addr.ip();
        let is_local = is_local_network(peer_ip);

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
                        "Server: Try connecting to WebSocket at ws[s]://<host>:<port>/ws or check /status.",
                    ),
                    "/status" => (StatusCode::OK, "Server Status: Online"),
                    "/index" => (StatusCode::OK, include_str!("../../static/web/index.html")),
                    _ => (StatusCode::NOT_FOUND, "404 Not Found"),
                };
                let body = Full::new(Bytes::from(body_text.to_string()));
                let response = HttpResponse::builder().status(status).body(body).unwrap();
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

fn is_local_network(addr: IpAddr) -> bool {
    // 1. Check for standard private ranges (RFC 1918) and loopback
    if addr.is_loopback() {
        return true;
    }

    // 2. Check for Link-Local Addresses (169.254.x.x)
    if let IpAddr::V4(ipv4) = addr {
        if ipv4.octets()[0] == 169 && ipv4.octets()[1] == 254 {
            return true;
        }
    }

    // 3. Check for IPv6 Unique Local Addresses (fc00::/7)
    if let IpAddr::V6(ipv6) = addr {
        if (ipv6.segments()[0] & 0xfe00) == 0xfc00 {
            return true;
        }
    }

    false
}

/// Runs the standard, unencrypted HTTP/WS server loop.
async fn run_http_server(port: u16) -> bool {
    // Bind to the port
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await;
    if let Err(e) = listener {
        log_message(format!("Failed to bind to port {}: {:?}", port, e));
        return false;
    }
    let listener = listener.unwrap();
    log_message(format!(
        "Standard Server listening for HTTP and WS on 0.0.0.0:{}",
        port
    ));

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                std::result::Result::Ok((stream, addr)) => {
                    let service = HttpService { peer_addr: addr };
                    let io = TokioIo::new(stream);

                    tokio::spawn(async move {
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

/// Runs the encrypted HTTPS/WSS server loop using the provided TLS config.
async fn run_tls_server(port: u16, tls_config: Arc<ServerConfig>) -> bool {
    let acceptor = TlsAcceptor::from(tls_config);

    // Bind to the port
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await;
    if let Err(e) = listener {
        log_message(format!("Failed to bind to port {}: {:?}", port, e));
        return false;
    }
    let listener = listener.unwrap();
    log_message(format!(
        "Encrypted Server listening for HTTPS and WSS on 0.0.0.0:{}",
        port
    ));

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                std::result::Result::Ok((stream, addr)) => {
                    let service = HttpService { peer_addr: addr };
                    let acceptor = acceptor.clone();

                    tokio::spawn(async move {
                        // Perform TLS handshake
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

pub async fn start(port: u16) -> bool {
    let tls_result = load_tls_config();

    match tls_result {
        Ok(Some(tls_config)) => {
            // Certificates found and config loaded successfully, run the TLS server
            run_tls_server(port, tls_config).await
        }
        Ok(None) => {
            // Certificates not found, run the standard HTTP server
            run_http_server(port).await
        }
        Err(e) => {
            log_message(format!("Fatal error during TLS config load: {}", e));
            // Error, server cannot start
            false
        }
    }
}
fn calculate_accept_key(key: &str) -> String {
    let websocket_guid = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut sha1 = Sha1::new();
    sha1.update(key.as_bytes());
    sha1.update(websocket_guid.as_bytes());
    let result = sha1.finalize();
    STANDARD.encode(result) // Base64 encode the result
}

/// Loads TLS config. Returns Ok(None) if cert files are not found, and an error if parsing fails.
fn load_tls_config() -> Result<Option<Arc<ServerConfig>>, Box<dyn Error>> {
    let cert_file_res = load_file_buf("certs", "cert.pem");
    let key_file_res = load_file_buf("certs", "cert.key");

    // Check if certificate files are present. If not, return None.
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
