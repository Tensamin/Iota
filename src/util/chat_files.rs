use crate::util::file_util::{get_children, load_file, save_file};
use json::{self, JsonValue, array, object};
use sha2::digest::typenum::Add1;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use sys_info::loadavg;
use uuid::Uuid;

use crate::gui::log_panel::log_message;
use crate::langu::language_manager::format;

pub enum MessageState {
    Read,
    Received,
    Sending,
    Error,
}

impl MessageState {
    fn as_str(&self) -> &'static str {
        match self {
            MessageState::Read => "READ",
            MessageState::Received => "RECEIVED",
            MessageState::Sending => "SENDING",
            MessageState::Error => "ERROR",
        }
    }
}

pub struct ChatFiles;

impl ChatFiles {
    pub fn add_message(
        send_time: i64,
        storage_owner_is_sender: bool,
        storage_owner: Uuid,
        external_user: Uuid,
        message: &str,
    ) {
        let user_dir = format!("users/{}/chats/{}", storage_owner, external_user);

        if let Err(e) = fs::create_dir_all(&user_dir) {
            log_message(format!("Failed to create chat directory: {}", e));
            return;
        }

        let mut chunk_index = 0;
        let mut message_chunk = array![];

        // find latest chunk not full (max 800 msgs)
        loop {
            let file_name = format!("msgs_{}.json", chunk_index);
            let file_content = load_file(&user_dir, &file_name);

            if !file_content.is_empty() {
                if let Ok(current_chunk) = json::parse(&file_content) {
                    if current_chunk.is_array() && current_chunk.len() < 800 {
                        message_chunk = current_chunk;
                        break;
                    }
                } else {
                    log_message(format!("Failed to parse existing JSON file: {}", file_name));
                }
            } else {
                // New file, use empty array
                break;
            }

            chunk_index += 1;
            if chunk_index > 1000 {
                log_message(format!("Too many message chunks. Aborting add."));
                return;
            }
        }

        let json_obj = object! {
            "message_time" => send_time,
            "message_content" => message,
            "sender_is_me" => storage_owner_is_sender,
            "message_state" => MessageState::Sending.as_str()
        };

        if let Err(e) = message_chunk.push(json_obj) {
            log_message(format!("Failed to push new message into JSON array: {}", e));
            return;
        }

        let file_name = format!("msgs_{}.json", chunk_index);
        log_message(format!("Saving message to {}/{}", user_dir, file_name));
        save_file(&user_dir, &file_name, &message_chunk.dump());
    }
    pub fn change_message_state(
        storage_owner: Uuid,
        external_user: Uuid,
        timestamp: i64,
        new_state: MessageState,
    ) -> std::io::Result<()> {
        let user_dir = format!("users/{}/chats/{}", storage_owner, external_user);
        let path = Path::new(&user_dir);

        if !path.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(path)?;
        for entry in entries {
            let entry = entry?;
            let fname = entry.file_name();
            let fname_str = fname.to_string_lossy();

            if fname_str.starts_with("msgs_") && fname_str.ends_with(".json") {
                let file_content = load_file(&user_dir, &fname_str);
                if file_content.is_empty() {
                    continue;
                }

                if let Ok(mut chunk) = json::parse(&file_content) {
                    let mut modified = false;
                    for i in 0..chunk.len() {
                        if chunk[i]["message_time"].as_i64() == Some(timestamp) {
                            chunk[i]["message_state"] = JsonValue::from(new_state.as_str());
                            modified = true;
                            break;
                        }
                    }

                    if modified {
                        save_file(&user_dir, &fname_str, &chunk.dump());
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_messages(
        storage_owner: Uuid,
        external_user: Uuid,
        loaded_messages: i64,
        amount: i64,
    ) -> JsonValue {
        let mut messages = array![];

        let mut latest_chunk_index: i32 = -1;
        let files = get_children(&format!("users/{}/chats/{}", storage_owner, external_user));

        for entry in files {
            if let Some(num) = {
                entry
                    .strip_prefix("msgs_")
                    .and_then(|s| s.strip_suffix(".json"))
            } {
                if let Ok(index) = num.parse::<i32>() {
                    if index > latest_chunk_index {
                        latest_chunk_index = index;
                    }
                }
            }
        }

        if latest_chunk_index == -1 {
            return messages;
        }

        let mut to_skip = loaded_messages;
        let mut needed = amount;

        for chunk_index in (0..=latest_chunk_index).rev() {
            if needed == 0 {
                break;
            }
            let file_name = format!("msgs_{}.json", chunk_index);
            let file_content = load_file(
                &format!("users/{}/chats/{}", storage_owner, external_user),
                &file_name,
            );
            if file_content.is_empty() {
                continue;
            }
            if let Ok(chunk) = json::parse(&file_content) {
                for i in (0..chunk.len()).rev() {
                    if needed == 0 {
                        break;
                    }
                    if to_skip > 0 {
                        to_skip -= 1;
                        continue;
                    }
                    messages.push(chunk[i].clone()).unwrap();
                    needed -= 1;
                }
            }
        }

        messages
    }
}
