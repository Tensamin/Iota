use std::sync::Arc;

use crate::communities::community::Community;
use crate::data::communication::{CommunicationType, CommunicationValue, DataTypes};
use crate::gui::log_panel::log_message;
use axum::http::HeaderValue;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{HeaderMap, Response as HttpResponse, StatusCode};
use json::JsonValue;
use uuid::Uuid;

use crate::util::config_util::CONFIG;
use crate::{APP_STATE, communities::community_manager, users::user_manager};

pub async fn handle(
    path: &str,
    is_local: &bool,
    headers: HeaderMap<HeaderValue>,
    body_string: Option<String>,
) -> HttpResponse<Full<Bytes>> {
    if !is_local {
        return HttpResponse::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Full::new(Bytes::from("403 Forbidden".to_string())))
            .unwrap();
    }

    let path_parts: Vec<&str> = path.split("/").collect();
    let body: Option<JsonValue> = if body_string.is_some() {
        body_string.map(|s| json::parse(&s).unwrap())
    } else {
        None
    };
    let (status, content, body_text) = if path_parts.len() >= 3 {
        match path_parts[2] {
            "app_state" => (StatusCode::OK, "application/json", {
                let with = headers
                    .get("size")
                    .unwrap_or(&HeaderValue::from_static("50"))
                    .to_str()
                    .unwrap()
                    .to_string();
                let json = APP_STATE
                    .lock()
                    .unwrap()
                    .with_width(with.parse::<u16>().unwrap_or(50));
                json.to_json().to_string()
            }),
            "users" => (StatusCode::OK, "application/json", {
                if path_parts.len() >= 4 {
                    match path_parts[3] {
                        "add" => {
                            if body.is_none() {
                                "{\"type\":\"error\"}".to_string()
                            } else {
                                let username =
                                    body.unwrap()["username"].as_str().unwrap().to_string();
                                if let (Some(user), Some(_private_key)) =
                                    user_manager::create_user(&username).await
                                {
                                    let cv =
                                        CommunicationValue::new(CommunicationType::create_user)
                                            .add_data(DataTypes::user, user.frontend());
                                    cv.to_json().to_string()
                                } else {
                                    "{\"type\":\"error\"}".to_string()
                                }
                            }
                        }
                        "remove" => {
                            if body.is_none() {
                                "{\"type\":\"error\"}".to_string()
                            } else {
                                let uuid = Uuid::parse_str(body.unwrap()["uuid"].as_str().unwrap())
                                    .unwrap();
                                user_manager::remove_user(uuid);
                                "{}".to_string()
                            }
                        }
                        "get" => {
                            let users = user_manager::get_users();
                            let mut json = JsonValue::new_array();
                            for user in users {
                                let _ = json.push(user.frontend());
                            }
                            json.to_string()
                        }
                        _ => "{\"type\":\"error\"}".to_string(),
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
                if path_parts.len() >= 4 {
                    match path_parts[3] {
                        "add" => {
                            if body.is_none() {
                                "{\"type\":\"error\"}".to_string()
                            } else {
                                let name = body.unwrap()["name"].as_str().unwrap().to_string();
                                let community = Arc::new(Community::create(name).await);
                                community_manager::add_community(community).await;
                                "{\"type\":\"success\"}".to_string()
                            }
                        }
                        "remove" => {
                            if body.is_none() {
                                "{\"type\":\"error\"}".to_string()
                            } else {
                                let name = body.unwrap()["name"].as_str().unwrap().to_string();
                                community_manager::remove_community(&name).await;
                                "{\"type\":\"success\"}".to_string()
                            }
                        }
                        "get" => {
                            let communities = community_manager::get_communities().await;
                            let mut json = JsonValue::new_array();
                            for community in communities {
                                let _ = json.push(community.frontend().await);
                            }
                            json.to_string()
                        }
                        _ => "{\"type\":\"error\"}".to_string(),
                    }
                } else {
                    let communities = community_manager::get_communities().await;
                    let mut json = JsonValue::new_array();
                    for community in communities {
                        let _ = json.push(community.to_json().await);
                    }
                    json.to_string()
                }
            }),
            "settings" => (StatusCode::OK, "application/json", {
                if path_parts.len() >= 4 {
                    match path_parts[3] {
                        "set" => {
                            if let Some(key) = headers.get("key") {
                                if let Some(value) = headers.get("value") {
                                    let _ = CONFIG
                                        .lock()
                                        .await
                                        .config
                                        .insert(key.to_str().unwrap(), value.to_str().unwrap());
                                    "{\"type\":\"success\"}".to_string()
                                } else {
                                    "{\"type\":\"error\"}".to_string()
                                }
                            } else {
                                "{\"type\":\"error\"}".to_string()
                            }
                        }
                        "get" => CONFIG.lock().await.config.to_string(),
                        _ => "{\"type\":\"error\"}".to_string(),
                    }
                } else {
                    CONFIG.lock().await.config.to_string()
                }
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
