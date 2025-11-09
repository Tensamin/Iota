use json::{self, JsonValue};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Contact {
    pub user_id: Option<Uuid>,
    pub user_name: Option<String>,
    pub last_message_at: Option<i64>,
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
        }
    }
}

impl Contact {
    pub fn new(user_id: Uuid) -> Self {
        Contact {
            user_id: Some(user_id),
            user_name: None,
            last_message_at: None,
        }
    }
    pub fn set_last_message_at(&mut self, p0: i64) {
        self.last_message_at = Option::from(p0);
    }

    pub fn to_json(&self) -> JsonValue {
        let mut obj = JsonValue::new_object();
        if let Some(id) = &self.user_id {
            obj["user_id"] = JsonValue::from(id.to_string());
        }
        if let Some(name) = &self.user_name {
            obj["user_name"] = JsonValue::from(name.as_str());
        }
        if let Some(ts) = &self.last_message_at {
            obj["last_message_at"] = JsonValue::from(ts.to_string());
        }
        obj
    }
    pub fn from_json(o: &JsonValue) -> Contact {
        let user_id = o["user_id"].as_str().and_then(|s| Uuid::parse_str(s).ok());

        let user_name = o["user_name"].as_str().map(|s| s.to_string());

        let last_message_at = o["last_message_at"].as_i64();

        Contact {
            user_id,
            user_name,
            last_message_at,
        }
    }
}
