use crate::server::server::is_local_network;
use crate::util::config_util::CONFIG;
use axum::routing::{get, post};
use axum::{
    extract::{ConnectInfo, Json, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::sync::Arc;

pub fn api_router() -> axum::Router<bool> {
    axum::Router::new()
        .route("/shutdown", post(shutdown))
        .route("/reload", post(reload))
        .route("/users/add", post(users_add))
        .route("/users/remove", post(users_remove))
        .route("/users/get", get(users_get))
        .route("/communities/add", post(communities_add))
        .route("/communities/get", get(communities_get))
        .route("/settings/set", post(settings_set))
        .route("/settings/get", get(settings_get))
}
pub async fn settings_set(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(ssl): State<bool>,
    headers: HeaderMap,
) -> Response {
    if !is_allowed(addr, ssl) {
        return forbidden();
    }

    let key = headers.get("key").and_then(|v| v.to_str().ok());
    let value = headers.get("value").and_then(|v| v.to_str().ok());

    match (key, value) {
        (Some(k), Some(v)) => {
            let _ = CONFIG
                .write()
                .await
                .config
                .insert(&k.to_string(), v.to_string());

            success()
        }
        _ => error(),
    }
}

pub async fn settings_get(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(ssl): State<bool>,
) -> Response {
    if !is_allowed(addr, ssl) {
        return forbidden();
    }
    let config = CONFIG.read().await.config.clone();
    let serde_config: Value = serde_json::to_value(config.to_string()).unwrap();
    (StatusCode::OK, Json(serde_config)).into_response()
}
pub async fn communities_get(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(ssl): State<bool>,
) -> Response {
    if !is_allowed(addr, ssl) {
        return forbidden();
    }

    let communities = crate::communities::community_manager::get_communities().await;

    let mut list = Vec::new();

    for c in communities {
        let val = c.frontend().await;
        let s_val: Value = serde_json::to_value(val.to_string()).unwrap();
        list.push(s_val);
    }
    (StatusCode::OK, Json(list)).into_response()
}
pub async fn communities_add(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(ssl): State<bool>,
    Json(payload): Json<Value>,
) -> Response {
    if !is_allowed(addr, ssl) {
        return forbidden();
    }

    let name = payload["name"].as_str().unwrap_or("").to_string();
    let owner = payload["owner"].as_i64().unwrap_or(0);

    let community = Arc::new(crate::communities::community::Community::create(name, owner).await);

    crate::communities::community_manager::add_community(community).await;

    success()
}
pub async fn users_get(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(ssl): State<bool>,
) -> Response {
    if !is_allowed(addr, ssl) {
        return forbidden();
    }

    let users = crate::users::user_manager::get_users();

    let list: Vec<_> = users
        .into_iter()
        .map(|u| {
            let val = u.frontend();
            serde_json::to_value(val.to_string()).unwrap()
        })
        .collect();

    (StatusCode::OK, Json(list)).into_response()
}
pub async fn users_remove(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(ssl): State<bool>,
    Json(payload): Json<Value>,
) -> Response {
    if !is_allowed(addr, ssl) {
        return forbidden();
    }

    let uuid = payload.get("uuid").and_then(|v| v.as_i64()).unwrap_or(0);

    crate::users::user_manager::remove_user(uuid);
    crate::users::user_manager::save_users();

    success()
}
pub async fn users_add(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(ssl): State<bool>,
    Json(payload): Json<Value>,
) -> Response {
    if !is_allowed(addr, ssl) {
        return forbidden();
    }

    let username = match payload.get("username").and_then(|v| v.as_str()) {
        Some(u) => u,
        None => return error(),
    };

    if let (Some(user), Some(_)) = crate::users::user_manager::create_user(username).await {
        let val = user.frontend();
        let s_val: Value = serde_json::to_value(val.to_string()).unwrap();
        (StatusCode::OK, Json(s_val)).into_response()
    } else {
        error()
    }
}
pub async fn shutdown(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(ssl): State<bool>,
) -> Response {
    if !is_allowed(addr, ssl) {
        return forbidden();
    }

    *crate::SHUTDOWN.write().await = true;
    success()
}

pub async fn reload(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(ssl): State<bool>,
) -> Response {
    if !is_allowed(addr, ssl) {
        return forbidden();
    }

    *crate::SHUTDOWN.write().await = true;
    *crate::RELOAD.write().await = true;

    success()
}

fn forbidden() -> Response {
    (StatusCode::FORBIDDEN, "403 Forbidden").into_response()
}

fn success() -> Response {
    Json(json!({ "type": "success" })).into_response()
}

fn error() -> Response {
    Json(json!({ "type": "error" })).into_response()
}

fn is_allowed(addr: SocketAddr, ssl: bool) -> bool {
    is_local_network(addr.ip()) || ssl
}
