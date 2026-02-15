use actix_web::{HttpRequest, HttpResponse, web};
use json::JsonValue;
use std::path::{Path, PathBuf};

use crate::util::file_util::load_file_vec;

fn codec_for_ext(ext: &str) -> &'static str {
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

pub async fn handle(req: HttpRequest, body: web::Bytes) -> HttpResponse {
    // Optional JSON parsing
    let _body_json: Option<JsonValue> = if !body.is_empty() {
        json::parse(std::str::from_utf8(&body).unwrap_or("")).ok()
    } else {
        None
    };

    let req_path = req.path().trim_start_matches('/');

    // 1️⃣ Resolve the filesystem path
    let mut fs_path = PathBuf::from("web");

    // Boolean P: no path provided → redirect to index.html
    if req_path.is_empty() {
        fs_path.push("index.html");
    } else {
        fs_path.extend(req_path.split('/'));
    }

    // Boolean D: path is directory → serve index.html inside
    if fs_path.is_dir() {
        fs_path.push("index.html");
    }

    // Boolean E: extension provided
    let ext_opt = fs_path.extension().and_then(|e| e.to_str());
    let mut final_name = fs_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    if ext_opt.is_none() {
        // No extension provided → try HTML
        if final_name.is_empty() {
            final_name = "index.html".to_string();
        } else {
            final_name.push_str(".html");
        }
    }

    let content_type = codec_for_ext(
        fs_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("html"),
    );

    let dir = fs_path.parent().unwrap_or(Path::new("web"));

    // 2️⃣ Try to load the resolved file
    match load_file_vec(dir.to_str().unwrap_or("web"), &final_name) {
        Ok(content) => HttpResponse::Ok().content_type(content_type).body(content),

        Err(_) => {
            // For static assets, return plain 404
            let ext = fs_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if matches!(ext, "js" | "css" | "woff2") {
                return HttpResponse::NotFound()
                    .content_type("text/plain")
                    .body("Not found");
            }

            // 3️⃣ Try to serve 404.html from web folder
            let fallback = load_file_vec("web", "404.html")
                .unwrap_or_else(|_| include_bytes!("../../static/web/404.html").to_vec());

            HttpResponse::NotFound()
                .content_type("text/html; charset=utf-8")
                .body(fallback)
        }
    }
}
