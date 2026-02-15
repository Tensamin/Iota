use crate::gui::log_panel::log_message;
use crate::server::api::api_router;
use crate::server::socket::handle;
use crate::server::web_path_parser::codec_for_ext;
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
use futures_util::StreamExt;
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
        .route("/ws/{*path}", get(ws_handler))
        .nest("/api", api_router())
        .route("/{*path}", any(static_handler))
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
    let (sender, receiver) = socket.split();
    handle(path, sender, receiver);
}

async fn static_handler(Path(path): Path<String>) -> Response {
    let mut parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let name = if parts.is_empty() {
        "index.html"
    } else {
        let last_part = parts.last().unwrap();
        if last_part.contains('.') {
            parts.pop().unwrap()
        } else {
            "index.html"
        }
    };

    let path_prefix = parts.join("/");
    let content_result = load_file_vec(&format!("web/{}/", path_prefix), name);

    match content_result {
        Ok(content) => {
            let mime = codec_for_ext(name);
            let mut headers = HeaderMap::new();
            headers.insert(axum::http::header::CONTENT_TYPE, mime.parse().unwrap());
            (StatusCode::OK, headers, content).into_response()
        }
        Err(_) => {
            let content_404 = load_file_vec("web", "404.html").unwrap_or_default();
            let mut headers = HeaderMap::new();
            headers.insert(
                axum::http::header::CONTENT_TYPE,
                "text/html; charset=utf-8".parse().unwrap(),
            );
            (StatusCode::NOT_FOUND, headers, content_404).into_response()
        }
    }
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
