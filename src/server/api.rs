use crate::server::server::is_local_network;
use axum::routing::{get, post};
use axum::{
    extract::{ConnectInfo, Json, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use json::JsonValue;
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
) -> impl IntoResponse {
    if !is_allowed(addr, ssl) {
        return forbidden();
    }

    let key = headers.get("key").and_then(|v| v.to_str().ok());
    let value = headers.get("value").and_then(|v| v.to_str().ok());

    match (key, value) {
        (Some(k), Some(v)) => {
            crate::util::config_util::CONFIG
                .write()
                .await
                .config
                .insert(k, v);

            success()
        }
        _ => error(),
    }
}

pub async fn settings_get(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(ssl): State<bool>,
) -> impl IntoResponse {
    if !is_allowed(addr, ssl) {
        return forbidden();
    }

    (StatusCode::OK, Json(CONFIG.read().await.config.clone())).into_response()
}
pub async fn communities_get(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(ssl): State<bool>,
) -> impl IntoResponse {
    if !is_allowed(addr, ssl) {
        return forbidden();
    }

    let communities = crate::communities::community_manager::get_communities().await;

    let mut list = JsonValue::new_array();

    for c in communities {
        list.push(c.frontend().await);
    }

    (StatusCode::OK, Json(list)).into_response()
}
pub async fn communities_add(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(ssl): State<bool>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
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
) -> impl IntoResponse {
    if !is_allowed(addr, ssl) {
        return forbidden();
    }

    let users = crate::users::user_manager::get_users();

    let list: Vec<_> = users.into_iter().map(|u| u.frontend()).collect();

    Json(list)
}
pub async fn users_remove(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(ssl): State<bool>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
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
) -> impl IntoResponse {
    if !is_allowed(addr, ssl) {
        return forbidden();
    }

    let username = match payload.get("username").and_then(|v| v.as_str()) {
        Some(u) => u,
        None => return error().into_response(),
    };

    if let (Some(user), Some(_)) = crate::users::user_manager::create_user(username).await {
        (StatusCode::OK, Json(user.frontend())).into_response()
    } else {
        error()
    }
}
pub async fn shutdown(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(ssl): State<bool>,
) -> impl IntoResponse {
    if !is_allowed(addr, ssl) {
        return forbidden();
    }

    *crate::SHUTDOWN.write().await = true;
    success()
}

pub async fn reload(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(ssl): State<bool>,
) -> impl IntoResponse {
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
