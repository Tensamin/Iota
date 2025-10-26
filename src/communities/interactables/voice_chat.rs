use crate::{
    communities::{community::Community, interactables::interactable::Interactable},
    data::communication::{CommunicationType, CommunicationValue},
};
use json::JsonValue;
use std::any::Any;
use std::sync::Arc;
use uuid::Uuid;
pub enum CallUserState {
    Active,
    Muted,
    Deafed,
}

pub struct CallUser {
    user_id: Uuid,
    user_state: CallUserState,
    streaming: bool,
}

pub struct VoiceChat {
    name: String,
    path: String,
    community: Arc<Community>,
    users: Vec<CallUser>,
}
impl VoiceChat {
    pub fn new() -> VoiceChat {
        VoiceChat {
            name: String::new(),
            path: String::new(),
            community: Arc::new(Community::new()),
            users: Vec::new(),
        }
    }
}
impl Interactable for VoiceChat {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn get_codec(&self) -> String {
        "voice".to_string()
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
    fn set_path(&mut self, path: String) {
        self.path = path;
    }
    fn get_community(&self) -> &Arc<Community> {
        &self.community
    }
    fn set_community(&mut self, community: Arc<Community>) {
        self.community = community;
    }
    fn get_name(&self) -> &String {
        &self.name
    }
    fn get_path(&self) -> &String {
        &self.path
    }
    fn get_total_path(&self) -> String {
        String::new() + &self.path + "/" + &self.name
    }
    fn get_data(&self) -> JsonValue {
        JsonValue::new_object()
    }
    fn run_function(&self, cv: CommunicationValue) -> CommunicationValue {
        CommunicationValue::new(CommunicationType::error)
    }
    fn to_json(&self) -> JsonValue {
        let mut v = JsonValue::new_object();
        v
    }
    fn load(&mut self, community: Arc<Community>, path: String, name: String, json: &JsonValue) {
        self.community = community;
        self.name = name;
        self.path = path;
    }
}
