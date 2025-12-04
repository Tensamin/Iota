use crate::{ACTIVE_TASKS, RELOAD, SHUTDOWN, gui::tui::UNIQUE, util::config_util::CONFIG};
use crossterm::event::{Event, KeyCode, read};
use crossterm::event::{KeyEvent, KeyModifiers};
use tokio::{self};

pub fn setup_input_handler() {
    tokio::spawn(async move {
        {
            ACTIVE_TASKS
                .lock()
                .unwrap()
                .push("Input Handler".to_string());
        }
        while let Ok(event) = read() {
            if *SHUTDOWN.read().await {
                break;
            }
            if let Event::Key(key) = event {
                handle_input(key).await;
            }
            if *SHUTDOWN.read().await {
                break;
            }
        }
        {
            ACTIVE_TASKS
                .lock()
                .unwrap()
                .retain(|t| !t.eq(&"Input Handler".to_string()));
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

            CONFIG.write().await.change("password", password);
            CONFIG.write().await.update();
            *UNIQUE.write().await = true;
        }
        (KeyCode::Char(c), KeyModifiers::NONE) => {
            let password: String = {
                let cfg = CONFIG.read().await;
                cfg.get("password").as_str().unwrap_or("").to_string()
            };
            let password = &format!("{}{}", password, c);

            CONFIG.write().await.change("password", password);
            CONFIG.write().await.update();
            *UNIQUE.write().await = true;
        }
        (KeyCode::Char(c), KeyModifiers::SHIFT) => {
            let password: String = {
                let cfg = CONFIG.read().await;
                cfg.get("password").as_str().unwrap_or("").to_string()
            };
            let password = &format!("{}{}", password, c);

            CONFIG.write().await.change("password", password);
            CONFIG.write().await.update();
            *UNIQUE.write().await = true;
        }
        _ => {}
    }
}
