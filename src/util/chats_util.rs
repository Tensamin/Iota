use std::string::String;
use json::{self, array, JsonValue};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use axum::Json;
use uuid::Uuid;

use crate::users::contact;
use crate::users::contact::Contact;
// assuming you have a Contact struct in a module

pub struct ChatsUtil;

impl ChatsUtil {
    fn load_file(dir: &str, file_name: &str) -> String {
        let path = Path::new(dir).join(file_name);
        if let Ok(mut f) = File::open(&path) {
            let mut content = String::new();
            let _ = f.read_to_string(&mut content);
            content
        } else {
            String::new()
        }
    }

    fn save_file(dir: &str, file_name: &str, content: &str) -> std::io::Result<()> {
        fs::create_dir_all(dir)?;
        let path = Path::new(dir).join(file_name);
        let mut file = File::create(path)?;
        file.write_all(content.as_bytes())
    }

    pub fn mod_user(storage_owner: Uuid, contact: &Contact){
        let dir = format!("users/{}/contacts/", storage_owner);
        let file_name = "contacts.json";
        let s = Self::load_file(&dir, file_name);

        let mut contacts = if !s.is_empty() {
            json::parse(&s).unwrap_or(array![])
        } else {
            array![]
        };

        for i in 0..contacts.len() {
            if contacts[i]["userID"].as_str() == Some(&contact.user_id.unwrap().to_string()) {
                contacts.remove(stringify!("{}", i));
                break;
            }
        }

        contacts.push(contact.to_json()).unwrap();
        Self::save_file(&dir, file_name, &contacts.dump());
    }

    pub fn get_user(storage_owner: Uuid, user_id: Uuid) -> Option<Contact> {
        let dir = format!("users/{}/contacts/", storage_owner);
        let file_name = "contacts.json";
        let s = Self::load_file(&dir, file_name);
        if s.is_empty() {
            return None;
        }

        if let Ok(contacts) = json::parse(&s) {
            for i in 0..contacts.len() {
                if let Some(uid) = contacts[i]["userID"].as_str() {
                    if Uuid::parse_str(uid).ok()? == user_id {
                        return Option::from(Contact::from_json(&contacts[i]));
                    }
                }
            }
        }
        None
    }

    pub fn get_users(storage_owner: Uuid) -> JsonValue {
        let dir = format!("/users/{}/contacts/", storage_owner);
        let file_name = "contacts.json";
        let s = Self::load_file(&dir, file_name);

        let mut contacts_out = array![];
        if !s.is_empty() {
            if let Ok(contacts) = json::parse(&s) {
                for i in 0..contacts.len() {
                    let c = Contact::from_json(&contacts[i]);
                    contacts_out.push(c.to_json()).unwrap();
                }
            }
        }

        contacts_out
    }
}