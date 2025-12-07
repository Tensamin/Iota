use crate::CONFIG;
use crate::data::communication::{CommunicationType, CommunicationValue, DataTypes};
use crate::users::user_profile::UserProfile;
use json::JsonValue;
use json::number::Number;
use reqwest::header::CONTENT_TYPE;
use reqwest::{Client, Response};
use std::time::Duration;
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub created_at: i64,
    pub username: String,
    pub display: String,
    pub avatar: String,
    pub about: String,
    pub status: String,
    pub public_key: String,
    pub sub_level: i32,
    pub sub_end: i32,
}

fn client() -> Client {
    Client::builder()
        .connect_timeout(Duration::from_secs(100))
        .timeout(Duration::from_secs(150))
        .build()
        .unwrap()
}

pub async fn unregister_user(user_id: i64, reset_token: &str) -> Option<bool> {
    let url = format!("https:/auth.tensamin.net/api/delete/{}", user_id);
    let client = client();

    let mut payload = JsonValue::new_object();
    payload["reset_token"] = reset_token.into();

    let res = client
        .post(&url)
        .header(CONTENT_TYPE, "application/json")
        .body(payload.dump())
        .send()
        .await
        .ok()?;
    let json = res.text().await.ok()?;
    let cv = CommunicationValue::from_json(&json);
    Option::from(cv.is_type(CommunicationType::success))
}

pub async fn get_user(user_id: i64) -> Option<AuthUser> {
    let url = format!("https://auth.tensamin.net/api/get/{}", user_id);
    let client = client();
    let res = client.get(&url).send().await.ok()?;
    let json = res.text().await.ok()?;

    let cv = CommunicationValue::from_json(&json);
    if cv.comm_type != CommunicationType::success {
        return None;
    }

    Some(AuthUser {
        created_at: cv
            .get_data(DataTypes::created_at)
            .unwrap()
            .to_string()
            .parse::<i64>()
            .unwrap_or(-1),
        username: cv.get_data(DataTypes::username).unwrap().to_string(),
        display: cv.get_data(DataTypes::display).unwrap().to_string(),
        avatar: cv.get_data(DataTypes::avatar).unwrap().to_string(),
        about: cv.get_data(DataTypes::about).unwrap().to_string(),
        status: cv.get_data(DataTypes::status).unwrap().to_string(),
        public_key: cv.get_data(DataTypes::public_key).unwrap().to_string(),
        sub_level: cv
            .get_data(DataTypes::sub_level)
            .unwrap()
            .to_string()
            .parse::<i32>()
            .unwrap_or(-1),
        sub_end: cv
            .get_data(DataTypes::sub_end)
            .unwrap()
            .to_string()
            .parse::<i32>()
            .unwrap_or(-1),
    })
}

pub async fn get_register() -> Option<i64> {
    let url = "https://auth.tensamin.net/api/register/init".to_string();
    let client = client();
    let res = client.get(&url).send().await.ok()?;
    let json = res.text().await.ok()?;

    let cv = CommunicationValue::from_json(&json);
    cv.get_data(DataTypes::user_id)
        .unwrap_or(&json::JsonValue::Number(Number::from(0)))
        .as_i64()
}

pub async fn complete_register(user_profile: &UserProfile, iota_id: &str) -> bool {
    let url = "https://auth.tensamin.net/api/register/complete";
    let client = client();

    let mut payload = JsonValue::new_object();
    payload["id"] = user_profile.user_id.into();
    payload["public_key"] = user_profile.public_key.clone().into();
    payload["private_key_hash"] = user_profile.private_key_hash.clone().into();
    payload["username"] = user_profile.username.clone().into();
    payload["iota_id"] = iota_id.into();
    payload["reset_token"] = user_profile.reset_token.clone().into();

    let res = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .body(payload.to_string())
        .send()
        .await
        .unwrap();

    let body = res.text().await.unwrap();

    CommunicationValue::from_json(&body).is_type(CommunicationType::success)
}

pub async fn migrate_user(user_profile: &mut UserProfile) -> bool {
    let url = format!(
        "https://auth.tensamin.net/api/change/iota-id/{}",
        user_profile.user_id
    );
    let client = client();

    let mut payload = JsonValue::new_object();
    payload["iota_id"] = JsonValue::String(CONFIG.read().await.get_iota_id().to_string());
    payload["reset_token"] = user_profile.reset_token.clone().into();
    payload["new_token"] = user_profile.randomize_reset_token().into();

    let res = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .body(payload.dump())
        .send()
        .await;

    match res {
        Ok(resp) => handle_response(resp).await,
        Err(_) => false,
    }
}

async fn handle_response(resp: Response) -> bool {
    match resp.text().await {
        Ok(text) => {
            let cv = CommunicationValue::from_json(&text.to_string());
            cv.comm_type == CommunicationType::success
        }
        Err(_) => false,
    }
}
