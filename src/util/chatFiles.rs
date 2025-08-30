
use json::{self, Value};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn add_message(
        send_time: i64,
        storage_owner_is_sender: bool,
        storage_owner: Uuid,
        external_user: Uuid,
        message: &str,
    ) -> std::io::Result<()> {
        let user_dir = format!("users/{}/chats/{}", storage_owner, external_user);

        let mut chunk_index = 0;
        let mut message_chunk = array![];

        // find latest chunk not full (max 800 msgs)
        loop {
            let file_name = format!("msgs_{}.json", chunk_index);
            let file_content = Self::load_file(&user_dir, &file_name);

            if !file_content.is_empty() {
                if let Ok(current_chunk) = json::parse(&file_content) {
                    if current_chunk.len() < 800 {
                        message_chunk = current_chunk;
                        break;
                    }
                }
            } else {
                break;
            }
            chunk_index += 1;
        }

        let json_obj = object! {
            "message_time" => send_time,
            "message_content" => message,
            "sender_is_me" => storage_owner_is_sender,
            "message_state" => MessageState::Sending.as_str()
        };
        message_chunk.push(json_obj).unwrap();

        Self::save_file(&user_dir, &format!("msgs_{}.json", chunk_index), &message_chunk.dump())
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
                let file_content = Self::load_file(&user_dir, &fname_str);
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
                        Self::save_file(&user_dir, &fname_str, &chunk.dump())?;
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
        loaded_messages: usize,
        amount: usize,
    ) -> JsonValue {
        let user_dir = format!("users/{}/chats/{}", storage_owner, external_user);
        let path = Path::new(&user_dir);
        let mut messages = array![];

        if !path.exists() {
            return messages;
        }

        let mut latest_chunk_index: i32 = -1;
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let fname = entry.file_name();
                let fname_str = fname.to_string_lossy();
                if fname_str.starts_with("msgs_") && fname_str.ends_with(".json") {
                    if let Some(num) = fname_str
                        .strip_prefix("msgs_")
                        .and_then(|s| s.strip_suffix(".json"))
                    {
                        if let Ok(index) = num.parse::<i32>() {
                            if index > latest_chunk_index {
                                latest_chunk_index = index;
                            }
                        }
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
            let file_content = Self::load_file(&user_dir, &file_name);
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

fn main() {
    let owner = Uuid::new_v4();
    let external = Uuid::new_v4();

    ChatFiles::add_message(1234567890, true, owner, external, "Hello!").unwrap();
    ChatFiles::change_message_state(owner, external, 1234567890, MessageState::Read).unwrap();
    let msgs = ChatFiles::get_messages(owner, external, 0, 10);

    println!("Messages: {}", msgs.dump());
}
