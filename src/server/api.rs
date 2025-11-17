use crate::gui::log_panel::log_message;
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

    // Collect path parts into a vector to avoid iterator consumption issues
    let path_parts: Vec<&str> = path.split("/").collect();

    // path_parts[0] is empty (before first /), path_parts[1] should be "api", path_parts[2] is the endpoint
    let (status, content, body_text) = if path_parts.len() >= 3 {
        match path_parts[2] {
            "app_state" => (StatusCode::OK, "application/json", {
                let json = APP_STATE.lock().unwrap().clone();
                json.to_json().to_string()
            }),
            "users" => (StatusCode::OK, "application/json", {
                // Check for sub-endpoints like /api/users/add, /api/users/get, etc.
                if path_parts.len() >= 4 {
                    match path_parts[3] {
                        "add" => "{}".to_string(),
                        "remove" => "{}".to_string(),
                        "get" => {
                            let users = user_manager::get_users();
                            let mut json = JsonValue::new_array();
                            for user in users {
                                let _ = json.push(user.to_json());
                            }
                            json.to_string()
                        }
                        _ => "{}".to_string(),
                    }
                } else {
                    // Default: return all users
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

            _ => {
                log_message(format!("Unknown API endpoint: {}", path));
                (
                    StatusCode::NOT_FOUND,
                    "application/json",
                    "404 Not Found".to_string(),
                )
            }
        }
    } else {
        log_message(format!("Invalid API path: {}", path));
        (
            StatusCode::NOT_FOUND,
            "application/json",
            "404 Not Found".to_string(),
        )
    };
    let body = Full::new(Bytes::from(body_text.to_string()));
    HttpResponse::builder()
        .header("Content-Type", content)
        .status(status)
        .body(body)
        .unwrap()
}
