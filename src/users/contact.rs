use json::{self, JsonValue};
use std::time::{SystemTime, UNIX_EPOCH};
use axum::Json;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Contact {
    pub user_id: Option<Uuid>,
    pub user_name: Option<String>,
    pub last_message_at: Option<i64>,
    pub user_status: UserStatus,
    pub about: Option<String>,
}

#[derive(Debug, Clone)]
pub enum UserStatus {
    Online,
    DoNotDisturb,
    WC,
    Away,
    UserOffline,
    IotaOffline,
}

impl Default for Contact {
    fn default() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        Contact {
            user_id: None,
            user_name: None,
            last_message_at: Some(now),
            user_status: UserStatus::UserOffline,
            about: None,
        }
    }
}

impl Contact {
    pub fn new_with_time(last_message_at: i64, user_id: Uuid) -> Self {
        Contact {
            user_id: Some(user_id),
            user_name: None,
            last_message_at: Some(last_message_at),
            user_status: UserStatus::UserOffline,
            about: None,
        }
    }

    pub fn new(user_id: Uuid) -> Self {
        Contact {
            user_id: Some(user_id),
            user_name: None,
            last_message_at: None,
            user_status: UserStatus::UserOffline,
            about: None,
        }
    }
    pub fn set_last_message_at(&mut self, p0: i64) {
        self.last_message_at = Option::from(p0);
    }

    pub fn to_json(&self) -> JsonValue {
        let mut obj = JsonValue::new_object();
        if let Some(id) = &self.user_id {
            obj["userID"] = JsonValue::from(id.to_string());
        }
        if let Some(name) = &self.user_name {
            obj["userName"] = JsonValue::from(name.as_str());
        }
        if let Some(ts) = &self.last_message_at {
            obj["lastMessageAt"] = JsonValue::from(ts.to_string());
        }
        obj
    }

    pub fn from_string(s: &str) -> Contact {
        let parsed: JsonValue = JsonValue::from(s);
        Self::from_json(&parsed)
    }

    pub fn from_json(o: &JsonValue) -> Contact {
        let user_id = o["userID"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok());

        let user_name = o["userName"].as_str().map(|s| s.to_string());

        let last_message_at = o["lastMessageAt"].as_i64();

        Contact {
            user_id,
            user_name,
            last_message_at,
            user_status: UserStatus::UserOffline, // default
            about: None,
        }
    }
    pub fn info(&self) -> JsonValue {
        let mut obj = self.to_json();
        if let Some(id) = &self.user_id {
            obj["userID"] = JsonValue::from(id.to_string());
        }
        if let Some(name) = &self.user_name {
            obj["userName"] = JsonValue::from(name.as_str());
        }
        obj
    }

    // getters & setters
    pub fn get_about(&self) -> Option<&String> {
        self.about.as_ref()
    }

    pub fn set_about(&mut self, about: String) {
        self.about = Some(about);
    }

    pub fn get_user_id(&self) -> Option<Uuid> {
        self.user_id
    }

    pub fn set_user_id(&mut self, id: Uuid) {
        self.user_id = Some(id);
    }

    pub fn get_user_name(&self) -> Option<&String> {
        self.user_name.as_ref()
    }

    pub fn set_user_name(&mut self, name: String) {
        self.user_name = Some(name);
    }

    pub fn get_user_status(&self) -> &UserStatus {
        &self.user_status
    }

    pub fn set_user_status(&mut self, status: UserStatus) {
        self.user_status = status;
    }
}
