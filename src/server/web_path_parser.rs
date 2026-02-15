use std::path::{Path, PathBuf};

use axum::http::HeaderValue;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{HeaderMap, Response as HttpResponse, StatusCode};
use json::JsonValue;

use crate::util::file_util::load_file_vec;

pub fn codec_for_ext(ext: &str) -> &'static str {
    match ext {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

pub async fn handle(
    path: &str,
    _headers: HeaderMap<HeaderValue>,
    body_string: Option<String>,
) -> HttpResponse<Full<Bytes>> {
    let path_parts: Vec<&str> = path.split("/").filter(|s| !s.is_empty()).collect();

    let _body: Option<JsonValue> = if body_string.is_some() {
        if let Ok(body_json) = json::parse(&body_string.unwrap()) {
            Some(body_json)
        } else {
            None
        }
    } else {
        None
    };

    let req_path = path.trim_start_matches('/');
    let mut fs_path = PathBuf::from("web");

    if req_path.is_empty() {
        fs_path.push("index.html");
    } else {
        fs_path.extend(req_path.split('/'));
    }

    if fs_path.is_dir() {
        fs_path.push("index.html");
    }

    let ext = fs_path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let codec = codec_for_ext(ext);

    let dir = fs_path.parent().unwrap_or(Path::new("web"));
    let name = fs_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("index.html");

    let content = load_file_vec(dir.to_str().unwrap_or("web"), name);
    if let Ok(content) = content {
        HttpResponse::builder()
            .status(StatusCode::OK)
            .header("content-type", codec)
            .body(Full::new(Bytes::from(content)))
            .unwrap()
    } else {
        if matches!(ext, "js" | "css" | "woff2") {
            return HttpResponse::builder()
                .status(StatusCode::NOT_FOUND)
                .header("content-type", "text/plain")
                .body(Full::new(Bytes::from("Not found")))
                .unwrap();
        }

        let fallback = load_file_vec("web", "404.html");
        let body = if let Ok(fallback) = fallback {
            if fallback.is_empty() {
                include_str!("../../static/web/404.html")
                    .as_bytes()
                    .to_vec()
            } else {
                fallback
            }
        } else {
            include_str!("../../static/web/404.html")
                .as_bytes()
                .to_vec()
        };

        HttpResponse::builder()
            .status(StatusCode::NOT_FOUND)
            .header("content-type", "text/html; charset=utf-8")
            .body(Full::new(Bytes::from(body)))
            .unwrap()
    }
}
