use crate::gui::log_panel::log_message;
use crate::server::api::{self, api_router};
use crate::server::socket::handle;
use crate::util::file_util::{load_file_buf, load_file_vec};
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
use tower::ServiceBuilder;

fn build_router(ssl: bool) -> Router {
    Router::new()
        .route("/ws/*path", get(ws_handler))
        .nest("/api/*path", api_router())
        .route("/*path", any(static_handler))
        .layer(ServiceBuilder::new())
        .with_state(ssl)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(path): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    log_message(format!("WS connection from {}", addr));

    ws.on_upgrade(move |socket| async move {
        handle_ws(socket, path).await;
    })
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
