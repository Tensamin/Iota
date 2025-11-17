use axum::http::HeaderValue;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{HeaderMap, Response as HttpResponse, StatusCode};
use json::JsonValue;

use crate::util::config_util::CONFIG;
use crate::{APP_STATE, communities::community_manager, users::user_manager};

pub async fn handle(
    path: &str,
    is_local: &bool,
    headers: HeaderMap<HeaderValue>,
) -> HttpResponse<Full<Bytes>> {
    if !is_local {
        return HttpResponse::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Full::new(Bytes::from("403 Forbidden".to_string())))
            .unwrap();
    }
    let mut path_parts = path.split("/");
    let (status, content, body_text) = match path_parts.nth(1).unwrap() {
        "app_state" => (StatusCode::OK, "application/json", {
            let json = APP_STATE.lock().unwrap().clone();
            json.to_json().to_string()
        }),
        "users" => (StatusCode::OK, "application/json", {
            if path_parts.nth(2).is_some() {
                if path_parts.nth(2).unwrap() == "add" {
                    "{}".to_string()
                } else if path_parts.nth(2).unwrap() == "remove" {
                    "{}".to_string()
                } else if path_parts.nth(2).unwrap() == "get" {
                    let users = user_manager::get_users();
                    let mut json = JsonValue::new_array();
                    for user in users {
                        let _ = json.push(user.to_json());
                    }
                    json.to_string()
                } else {
                    "{}".to_string()
                }
            } else {
                let users = user_manager::get_users();
                let mut json = JsonValue::new_array();
                for user in users {
                    let _ = json.push(user.to_json());
                }
                json.to_string()
            }
        }),
        "communities" => (StatusCode::OK, "application/json", {
            let communities = community_manager::get_communities().await;
            let mut json = JsonValue::new_array();
            for community in communities {
                let _ = json.push(community.to_json().await);
            }
            json.to_string()
        }),
        "settings" => (StatusCode::OK, "application/json", {
            CONFIG.lock().unwrap().config.to_string()
        }),

        _ => (
            StatusCode::NOT_FOUND,
            "application/json",
            "404 Not Found".to_string(),
        ),
    };
    let body = Full::new(Bytes::from(body_text.to_string()));
    HttpResponse::builder()
        .header("Content-Type", content)
        .status(status)
        .body(body)
        .unwrap()
}
