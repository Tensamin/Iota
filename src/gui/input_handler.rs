use std::time::Duration;

use crate::ACTIVE_TASKS;
use crate::{RELOAD, SHUTDOWN, gui::tui::UNIQUE, util::config_util::CONFIG};
// Switched to poll/read which are available by default in crossterm
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, poll, read};

use json::JsonValue;

pub fn setup_input_handler() {
    tokio::spawn(async move {
        {
            let mut tasks = ACTIVE_TASKS.lock().unwrap();
            tasks.push("Input Handler".to_string());
        }

        loop {
            {
                let should_shutdown = *SHUTDOWN.read().await;
                if should_shutdown {
                    break;
                }
            }

            let has_event = match poll(Duration::from_millis(100)) {
                Ok(true) => true,
                Ok(false) => false,
                Err(_) => false,
            };

            if has_event {
                match read() {
                    Ok(event) => {
                        if let Event::Key(key_event) = event {
                            handle_input(key_event).await;
                        }
                    }
                    Err(_) => (),
                }
            }
        }
        {
            let mut tasks = ACTIVE_TASKS.lock().unwrap();
            tasks.retain(|t| t != "Input Handler");
        }
    });
}

pub async fn handle_input(key: KeyEvent) {
    match (key.code, key.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            *SHUTDOWN.write().await = true;
        }
        (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
            {
                *RELOAD.write().await = true;
            }
            {
                *SHUTDOWN.write().await = true;
            }
        }
        (KeyCode::Backspace, KeyModifiers::NONE) => {
            let password: String = {
                let cfg = CONFIG.read().await;
                cfg.get("password").as_str().unwrap_or("").to_string()
            };
            let password: &str = &password;
            let password = match password.char_indices().next_back() {
                Some((i, _)) => &password[..i],
                None => password,
            };

            CONFIG
                .write()
                .await
                .change("password", JsonValue::String(password.to_string()));
            CONFIG.write().await.update();
            *UNIQUE.write().await = true;
        }
        (KeyCode::Char(c), KeyModifiers::NONE) => {
            let password: String = {
                let cfg = CONFIG.read().await;
                cfg.get("password").as_str().unwrap_or("").to_string()
            };
            let password = &format!("{}{}", password, c);

            CONFIG
                .write()
                .await
                .change("password", JsonValue::String(password.to_string()));
            CONFIG.write().await.update();
            *UNIQUE.write().await = true;
        }
        (KeyCode::Char(c), KeyModifiers::SHIFT) => {
            let password: String = {
                let cfg = CONFIG.read().await;
                cfg.get("password").as_str().unwrap_or("").to_string()
            };
            let password = &format!("{}{}", password, c);

            CONFIG
                .write()
                .await
                .change("password", JsonValue::String(password.to_string()));
            CONFIG.write().await.update();
            *UNIQUE.write().await = true;
        }
        _ => {}
    }
}
