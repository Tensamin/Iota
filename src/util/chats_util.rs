use json::{self, JsonValue, array};

use crate::gui::log_panel::log_message;
use crate::users::contact::Contact;
use crate::util::file_util::{load_file, save_file};

pub fn mod_user(storage_owner: i64, contact: &Contact) {
    let dir: &str = &format!("users/{}/contacts/", storage_owner);
    let s = load_file(dir, "contacts.json");

    let mut contacts = if !s.is_empty() {
        json::parse(&s).unwrap_or(array![])
    } else {
        array![]
    };

    for i in 0..contacts.len() {
        if contacts[i]["user_id"] == contact.user_id {
            contacts.array_remove(i);
            break;
        }
    }

    contacts.push(contact.to_json()).unwrap();
    save_file(&dir, "contacts.json", &contacts.dump());
}

pub fn get_user(storage_owner: i64, user_id: i64) -> Option<Contact> {
    let dir = format!("users/{}/contacts/", storage_owner);
    let s = load_file(&dir, "contacts.json");
    if s.is_empty() {
        return None;
    }

    if let Ok(contacts) = json::parse(&s) {
        for i in 0..contacts.len() {
            if let Some(uid) = contacts[i]["user_id"].as_i64() {
                if uid == user_id {
                    return Option::from(Contact::from_json(&contacts[i]));
                }
            }
        }
    }
    None
}

pub fn get_users(storage_owner: i64) -> JsonValue {
    let dir: &str = &format!("users/{}/contacts/", storage_owner);
    let s = load_file(dir, "contacts.json");

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
