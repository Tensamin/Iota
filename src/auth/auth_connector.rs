use std::time::Duration;
use std::collections::HashMap;
use uuid::Uuid;
use request::blocking::{Client, Response};
use request::header::CONTENT_TYPE;
use json::JsonValue;

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

    pub fn unregister_user(auth_server: &str, user_id: Uuid, reset_token: &str) -> bool {
        let url = format!("https://{}/api/delete/{}", auth_server, user_id);
        let client = Self::client();

        let mut payload = JsonValue::new_object();
        payload["reset_token"] = reset_token.into();

        let res = client.post(&url)
            .header(CONTENT_TYPE, "application/json")
            .body(payload.dump())
            .send();

        match res {
            Ok(resp) => Self::handle_response(resp),
            Err(_) => false,
        }
    }

    pub fn get_uuid(auth_server: &str, username: &str) -> Option<Uuid> {
        let url = format!("https://{}/api/get/uuid/{}", auth_server, username);
        let client = Self::client();
        let res = client.get(&url).send().ok()?;
        let json = res.text().ok()?;

        let cv = CommunicationValue::from_string(&json);
        if !cv.is_success() {
            return None;
        }
        cv.get_user_id()
    }

    pub fn get_user(auth_server: &str, user_id: Uuid) -> Option<AuthUser> {
        let url = format!("https://{}/api/get/{}", auth_server, user_id);
        let client = Self::client();
        let res = client.get(&url).send().ok()?;
        let json = res.text().ok()?;

        let cv = CommunicationValue::from_string(&json);
        if !cv.is_success() {
            return None;
        }

        Some(AuthUser {
            created_at: cv.get_number("created_at")? as i64,
            username: cv.get_string("username")?,
            display: cv.get_string("display")?,
            avatar: cv.get_string("avatar")?,
            about: cv.get_string("about")?,
            status: cv.get_string("status")?,
            public_key: cv.get_string("public_key")?,
            sub_level: cv.get_number("sub_level")? as i32,
            sub_end: cv.get_number("sub_end")? as i32,
        })
    }

    pub fn get_register(auth_server: &str) -> Option<Uuid> {
        let url = format!("https://{}/api/register/init/", auth_server);
        let client = Self::client();
        let res = client.get(&url).send().ok()?;
        let json = res.text().ok()?;

        let cv = CommunicationValue::from_string(&json);
        cv.get_user_id()
    }

    pub fn complete_register(auth_server: &str, user_profile: &UserProfile, iota_id: &str) -> bool {
        let url = format!("https://{}/api/register/complete/", auth_server);
        let client = Self::client();

        let mut payload = JsonValue::new_object();
        payload["uuid"] = user_profile.user_id.to_string().into();
        payload["public_key"] = user_profile.public_key.clone().into();
        payload["private_key_hash"] = user_profile.private_key_hash.clone().into();
        payload["username"] = user_profile.username.clone().into();
        payload["iota_id"] = iota_id.into();
        payload["reset_token"] = user_profile.reset_token.clone().into();

        let res = client.post(&url)
            .header(CONTENT_TYPE, "application/json")
            .body(payload.dump())
            .send();

        match res {
            Ok(resp) => Self::handle_response(resp),
            Err(_) => false,
        }
    }

    pub fn migrate_user(auth_server: &str, user_profile: &mut UserProfile, iota_id: &str) -> bool {
        let url = format!("https://{}/api/change/iota-id/{}", auth_server, user_profile.user_id);
        let client = Self::client();

        let mut payload = JsonValue::new_object();
        payload["iota_id"] = iota_id.into();
        payload["reset_token"] = user_profile.reset_token.clone().into();
        payload["new_token"] = user_profile.randomize_reset_token().into();

        let res = client.post(&url)
            .header(CONTENT_TYPE, "application/json")
            .body(payload.dump())
            .send();

        match res {
            Ok(resp) => Self::handle_response(resp),
            Err(_) => false,
        }
    }

    fn handle_response(resp: Response) -> bool {
        if let Ok(text) = resp.text() {
            let cv = CommunicationValue::from_string(&text);
            return cv.is_success();
        }
        false
    }
}

// --- Stubs for other modules ---
pub struct UserProfile {
    pub user_id: Uuid,
    pub public_key: String,
    pub private_key_hash: String,
    pub username: String,
    pub reset_token: String,
}
impl UserProfile {
    pub fn randomize_reset_token(&mut self) -> String {
        // stub: generate new token
        let new_tok = format!("{}-new", self.reset_token);
        self.reset_token = new_tok.clone();
        new_tok
    }
}

pub struct CommunicationValue;
impl CommunicationValue {
    pub fn from_string(s: &str) -> Self { Self }
    pub fn is_success(&self) -> bool { true }
    pub fn get_user_id(&self) -> Option<Uuid> { Some(Uuid::new_v4()) }
    pub fn get_string(&self, _k: &str) -> Option<String> { Some("demo".to_string()) }
    pub fn get_number(&self, _k: &str) -> Option<i64> { Some(123) }
}
