use json::{self, JsonValue};
use std::fs;
use std::path::Path;
use uuid::Uuid;

pub struct UserCommunityUtil;

impl UserCommunityUtil {
    pub fn add_community(storage_owner: Uuid, address: String, title: String, position: String) {
        let path = format!("users/{}/communities.json", storage_owner);
        let mut communities: JsonValue = Self::load_array(&path);

        let mut community = JsonValue::new_object();
        community["title"] = JsonValue::String(title);
        community["address"] = JsonValue::String(address);
        community["position"] = JsonValue::String(position);

        communities.push(community);
        Self::save_array(&path, communities);
    }

    pub fn remove_community(storage_owner: Uuid, community_address: String) {
        let path = format!("users/{}/communities.json", storage_owner);
        let mut communities = Self::load_array(&path);

        let mut new_array = JsonValue::new_array();
        for entry in communities.members() {
            if entry["address"].as_str() != Some(&community_address) {
                new_array.push(entry.clone()).unwrap();
            }
        }

        Self::save_array(&path, new_array);
    }

    pub fn get_communities(storage_owner: Uuid) -> JsonValue {
        let path = format!("users/{}/communities.json", storage_owner);
        Self::load_array(&path)
    }

    fn load_array(path: &str) -> JsonValue {
        if !Path::new(path).exists() {
            return JsonValue::new_object();
        }

        match fs::read_to_string(path) {
            Ok(content) => json::parse(&content).unwrap_or_else(|_| JsonValue::new_object()),
            Err(_) => JsonValue::new_object(),
        }
    }

    fn save_array(path: &str, arr: JsonValue) {
        if let Some(parent) = Path::new(path).parent() {
            let _ = fs::create_dir_all(parent);
        }

        let _ = fs::write(path, arr.pretty(3));
    }
}
