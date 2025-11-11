use crate::communities::interactables::category::Category;
use crate::communities::interactables::registry;
use crate::communities::{
    community_connection::CommunityConnection, interactables::interactable::Interactable,
};
use crate::data::communication::{CommunicationType, CommunicationValue};
use crate::util::file_util;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use json::JsonValue;
use json::object::Object;
use rand::RngCore;
use rand_core::OsRng;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use x448::{PublicKey, Secret};
/// Permissions
// uuid -> interactable/path/like/this/interactable_name:permission
//      -> role

/// Roles
// rolename -> interactable/path/like/this/interactable_name:permission
//          -> other/path/like/this/interactable_name:permission

pub struct Community {
    name: String,
    owner_id: Uuid,
    members: Vec<Uuid>,
    permissions: HashMap<Uuid, Vec<String>>,
    roles: HashMap<String, Vec<String>>,
    private_key: Secret,
    public_key: PublicKey,
    pub interactables: Arc<RwLock<Vec<Arc<Box<dyn Interactable>>>>>,
    pub connections: Arc<RwLock<HashMap<Uuid, Vec<Arc<CommunityConnection>>>>>,
}

impl Community {
    pub fn new() -> Self {
        let mut buf = [0u8; 56];
        let mut rng = OsRng;
        rng.fill_bytes(&mut buf);
        let private_key = Secret::from_bytes(&buf).unwrap();
        let public_key = PublicKey::from(&private_key);
        Community {
            name: String::new(),
            owner_id: Uuid::new_v4(),
            members: Vec::new(),
            permissions: HashMap::new(),
            roles: HashMap::new(),
            private_key,
            public_key,
            interactables: Arc::new(RwLock::new(Vec::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub async fn create(name: String) -> Self {
        let mut buf = [0u8; 56];
        let mut rng = OsRng;
        rng.fill_bytes(&mut buf);
        let private_key = Secret::from_bytes(&buf).unwrap();
        let public_key = PublicKey::from(&private_key);
        let c = Community {
            name,
            owner_id: Uuid::new_v4(),
            members: Vec::new(),
            permissions: HashMap::new(),
            roles: HashMap::new(),
            private_key,
            public_key,
            interactables: Arc::new(RwLock::new(Vec::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
        };
        c.save().await;
        c
    }

    pub fn add_member(&mut self, member_id: Uuid) {
        self.members.push(member_id);
    }

    pub fn remove_member(&mut self, member_id: Uuid) {
        self.members.retain(|id| *id != member_id);
    }
    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_private_key(&self) -> Secret {
        Secret::from_bytes(self.private_key.as_bytes()).unwrap()
    }
    pub fn get_public_key(&self) -> &PublicKey {
        &self.public_key
    }
    pub async fn add_connection(self: &Arc<Self>, other: Arc<CommunityConnection>) {
        let mut vec = self
            .connections
            .read()
            .await
            .get(&other.get_user_id().await.unwrap())
            .cloned()
            .unwrap_or_default();
        vec.push(other.clone());
        self.connections
            .write()
            .await
            .insert(other.get_user_id().await.unwrap(), vec);
    }
    pub async fn remove_connection(self: &Arc<Self>, other: Arc<CommunityConnection>) {
        let mut vec = self
            .connections
            .read()
            .await
            .get(&other.get_user_id().await.unwrap())
            .cloned()
            .unwrap_or_default();
        vec.retain(|conn| !Arc::ptr_eq(conn, &other));
        self.connections
            .write()
            .await
            .insert(other.get_user_id().await.unwrap(), vec);
    }
    pub async fn get_connections(&self) -> HashMap<Uuid, Vec<Arc<CommunityConnection>>> {
        self.connections.read().await.clone()
    }
    pub async fn get_connections_for_user(&self, user_id: Uuid) -> Vec<Arc<CommunityConnection>> {
        self.connections
            .read()
            .await
            .get(&user_id)
            .cloned()
            .unwrap_or_default()
    }
    pub async fn get_interactables(
        &self,
        user_id: Uuid,
    ) -> Vec<Arc<Box<dyn Interactable + 'static>>> {
        self.interactables.read().await.clone()
    }
    pub async fn add_interactable(self: &mut Arc<Self>, interactable: Arc<Box<dyn Interactable>>) {
        self.interactables.write().await.push(interactable);
    }
    pub async fn remove_interactable(
        self: &mut Arc<Self>,
        interactable: Arc<Box<dyn Interactable>>,
    ) {
        self.interactables
            .write()
            .await
            .retain(|i| !Arc::ptr_eq(i, &interactable));
    }
    pub async fn run_function(
        self: &mut Arc<Self>,
        user_id: Uuid,
        name: &str,
        path: &str,
        function: &str,
        cv: &CommunicationValue,
    ) -> CommunicationValue {
        if path.is_empty() {
            let target_interactables = &self.interactables.read().await.clone();
            for interactable in target_interactables.iter() {
                if interactable.get_name() == name {
                    if interactable.get_codec() == "category" {
                        return CommunicationValue::new(CommunicationType::error);
                    } else {
                        // cannot move a value of type dyn Interactable the size of dyn Interactable cannot be statically determined (rustc E0161)
                        return interactable.run_function(cv.clone()).await;
                    }
                }
            }
        } else {
            let target_interactables = &self.interactables.read().await.clone();
            for interactable in target_interactables.iter() {
                if interactable.get_name() == name {
                    if interactable.get_codec() == "category" {
                        let category: &Category =
                            interactable.as_any().downcast_ref::<Category>().unwrap();
                        // cannot move a value of type dyn Interactable the size of dyn Interactable cannot be statically determined (rustc E0161)
                        return category
                            .get_child(path.to_string(), name.to_string())
                            .unwrap()
                            .run_function(cv.clone())
                            .await;
                    } else {
                        return CommunicationValue::new(CommunicationType::error);
                    }
                }
            }
        }
        CommunicationValue::new(CommunicationType::add_chat)
    }

    pub async fn save(&self) {
        let mut json = Object::new();
        json.insert("name", JsonValue::String(self.name.clone()));
        json.insert("owner_id", JsonValue::String(self.owner_id.to_string()));

        json.insert(
            "private_key",
            JsonValue::String(STANDARD.encode(&self.private_key.as_bytes())),
        );
        json.insert(
            "public_key",
            JsonValue::String(STANDARD.encode(&self.public_key.as_bytes())),
        );

        file_util::save_file(
            &format!("communities/{}/", self.name),
            "config.json",
            &json.dump(),
        );

        let mut user_data = Object::new();
        for user in self.members.iter() {
            let mut data = JsonValue::new_object();

            let mut permissions = JsonValue::new_array();
            for perm in self.permissions.get(user).unwrap() {
                if let Ok(_) = permissions.push(perm.to_string()) {}
            }

            if let Ok(_) = data.insert("permissions", permissions) {}
            user_data.insert(&user.to_string(), data);
        }
        file_util::save_file(
            &format!("communities/{}/", self.name),
            "users.json",
            &user_data.dump(),
        );

        for interactable in self.interactables.read().await.clone().iter() {
            registry::save(interactable).await;
        }
    }
}
pub async fn load(name: &String) -> Option<Arc<Community>> {
    let file_contents = file_util::load_file(&format!("communities/{}/", name), "config.json");
    let json_content = json::parse(&file_contents).unwrap();

    let user_data = file_util::load_file(&format!("communities/{}/", name), "users.json");
    let user_json: JsonValue = json::parse(&user_data).unwrap();
    let mut users = Vec::new();
    let mut permissions: HashMap<Uuid, Vec<String>> = HashMap::new();

    for user in user_json.entries() {
        let (str, json): (&str, &JsonValue) = user;
        let perms_j = &json["permissions"];
        let perms = Vec::new();
        for _ in perms_j.entries() {
            // let perm_j = i.as_str().unwrap();
            // perms.push(perm_j.to_string());
        }

        let user_id = Uuid::parse_str(str).unwrap();

        users.push(user_id);
        permissions.insert(user_id, perms);
    }

    let role_data = file_util::load_file(&format!("communities/{}/", name), "roles.json");
    let roles: HashMap<String, Vec<String>> = HashMap::new();
    if let Ok(_) = json::parse(&role_data) {
        // Fill roles
    } else {
        return None;
    };

    let community = Community {
        name: json_content["name"].as_str().unwrap().to_string(),
        owner_id: Uuid::parse_str(json_content["owner_id"].as_str().unwrap()).unwrap(),
        members: users,
        roles,
        permissions,
        private_key: Secret::from_bytes(
            &STANDARD
                .decode(json_content["private_key"].as_str().unwrap())
                .unwrap(),
        )
        .unwrap(),
        public_key: PublicKey::from(
            &Secret::from_bytes(
                &STANDARD
                    .decode(json_content["private_key"].as_str().unwrap())
                    .unwrap(),
            )
            .unwrap(),
        ),
        interactables: Arc::new(RwLock::new(Vec::new())),
        connections: Arc::new(RwLock::new(HashMap::new())),
    };
    let mut comarc = Arc::new(community);

    let interactable_files: Vec<String> =
        file_util::get_children(&format!("communities/{}/interactables/", name));
    for file in interactable_files {
        if file.contains(".json") {
            let name = file.split('.').next().unwrap().to_string();
            let interactable: Box<dyn Interactable> =
                registry::load(comarc.clone(), String::new(), name).await;
            comarc.add_interactable(Arc::new(interactable)).await;
        }
    }
    Some(comarc)
}
