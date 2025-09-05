use std::time::Duration;
use uuid::{uuid, Uuid};
use reqwest::header::CONTENT_TYPE;
use json::JsonValue;
use reqwest::{Client, Response};
use crate::users::user_profile::UserProfile;
use crate::data::communication::{CommunicationType, CommunicationValue, DataTypes};

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

pub struct AuthConnector;

impl AuthConnector {
    fn client() -> Client {
        Client::builder()
            .connect_timeout(Duration::from_secs(100))
            .timeout(Duration::from_secs(150))
            .build()
            .unwrap()
    }

    pub async fn unregister_user(user_id: Uuid, reset_token: &str) -> Option<bool> {
        let url = format!("https:/auth.tensamin.methanium.net//api/delete/{}/", user_id);
        let client = Self::client();

        let mut payload = JsonValue::new_object();
        payload["reset_token"] = reset_token.into();

        let res = client.post(&url)
            .header(CONTENT_TYPE, "application/json")
            .body(payload.dump()).send().await.ok()?;
        let json = res.text().await.ok()?;
        let cv = CommunicationValue::from_json(&json);
        Option::from(cv.is_type(CommunicationType::Success))
        
    }

    pub async fn get_uuid(username: &str) -> Option<Uuid> {
        let url = format!("https://auth.tensamin.methanium.net/api/get/uuid/{}/", username);
        let client = Self::client();
        let res = client.get(&url).send().await.ok()?;
        let json = res.text().await.ok()?;
        let mut cv = CommunicationValue::from_json(&json);
        if !cv.is_type(CommunicationType::Success) {
            return None;
        }
        Uuid::parse_str(cv.get_data(DataTypes::UserId).unwrap()).ok()
    }
    
    pub async fn get_user(user_id: Uuid) -> Option<AuthUser> {
        let url = format!("https://auth.tensamin.methanium.net/api/get/{}/", user_id);
        let client = Self::client();
        let res = client.get(&url).send().await.ok()?;
        let json = res.text().await.ok()?;
        
        let mut cv = CommunicationValue::from_json(&json);
        if cv.comm_type != CommunicationType::Success {
            return None;
        }

        Some(AuthUser {
            created_at: cv.get_data(DataTypes::CreatedAt).unwrap().to_string().parse::<i64>().unwrap_or(-1),
            username: cv.get_data(DataTypes::Username).unwrap().to_string(),
            display: cv.get_data(DataTypes::Display).unwrap().to_string(),
            avatar: cv.get_data(DataTypes::Avatar).unwrap().to_string(),
            about: cv.get_data(DataTypes::About).unwrap().to_string(),
            status: cv.get_data(DataTypes::Status).unwrap().to_string(),
            public_key: cv.get_data(DataTypes::PublicKey).unwrap().to_string(),
            sub_level: cv.get_data(DataTypes::SubLevel).unwrap().to_string().parse::<i32>().unwrap_or(-1),
            sub_end: cv.get_data(DataTypes::SubEnd).unwrap().to_string().parse::<i32>().unwrap_or(-1),
        })
    }

    pub async fn get_register() -> Option<Uuid> {
        let url = "https://auth.tensamin.methanium.net/api/register/init/".to_string();
        let client = Self::client();
        let res = client.get(&url).send().await.ok()?;
        let json = res.text().await.ok()?;

        let mut cv = CommunicationValue::from_json(&json);
        Uuid::parse_str(cv.get_data(DataTypes::UserId).unwrap()).ok()
    }

    pub async fn complete_register(user_profile: &UserProfile, iota_id: &str) -> bool {
        let url = "https://auth.tensamin.methanium.net/api/register/complete/";
        let client = Self::client();

        let mut payload = JsonValue::new_object();
        payload["uuid"] = user_profile.user_id.to_string().into();
        payload["public_key"] = user_profile.public_key.clone().into();
        payload["private_key_hash"] = user_profile.private_key_hash.clone().into();
        payload["username"] = user_profile.username.clone().into();
        payload["iota_id"] = iota_id.into();
        payload["reset_token"] = user_profile.reset_token.clone().into();

        let res = client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .body(payload.dump())
            .send()
            .await;

        match res {
            Ok(resp) => Self::handle_response(resp).await,
            Err(_) => false,
        }
    }

    pub async fn migrate_user(user_profile: &mut UserProfile, iota_id: &str) -> bool {
        let url = format!(
            "https://auth.tensamin.methanium.net/api/change/iota-id/{}",
            user_profile.user_id
        );
        let client = Self::client();

        let mut payload = JsonValue::new_object();
        payload["iota_id"] = iota_id.into();
        payload["reset_token"] = user_profile.reset_token.clone().into();
        payload["new_token"] = user_profile.randomize_reset_token().into();

        let res = client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .body(payload.dump())
            .send()
            .await;

        match res {
            Ok(resp) => Self::handle_response(resp).await,
            Err(_) => false,
        }
    }

    async fn handle_response(resp: Response) -> bool {
        match resp.text().await {
            Ok(text) => {
                let cv = CommunicationValue::from_json(&text);
                cv.comm_type == CommunicationType::Success
            }
            Err(_) => false,
        }
    }

    
}