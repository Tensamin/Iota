use crate::{communities::community::Community, data::communication::CommunicationValue};
use json::JsonValue;
use std::any::Any;
use std::sync::Arc;

pub type InteractableFactory = fn() -> Box<dyn Interactable>;

pub trait Interactable: Send + Sync + Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn get_codec(&self) -> String;
    fn get_name(&self) -> &String;
    fn get_path(&self) -> &String;
    fn get_total_path(&self) -> String;
    fn set_name(&mut self, name: String);
    fn set_path(&mut self, path: String);
    fn get_community(&self) -> &Arc<Community>;
    fn set_community(&mut self, community: Arc<Community>);
    fn run_function(&self, cv: CommunicationValue) -> CommunicationValue;
    fn get_data(&self) -> JsonValue;
    fn to_json(&self) -> JsonValue;
    fn load(&mut self, community: Arc<Community>, path: String, name: String, json: &JsonValue);
}
