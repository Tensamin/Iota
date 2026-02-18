use crate::ACTIVE_TASKS;
use crate::gui::ui::UI;
use crate::{RELOAD, SHUTDOWN};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, poll, read};
use std::sync::Arc;
use std::time::Duration;

pub fn setup_input_handler(ui: Arc<UI>) {
    tokio::spawn(async move {
        ACTIVE_TASKS
            .lock()
            .unwrap()
            .push("Input Handler".to_string());

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
                            if key_event.kind == KeyEventKind::Press {
                                let ui_clone = ui.clone();
                                handle_input(key_event, ui_clone).await;
                            }
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

pub async fn handle_input(key: KeyEvent, ui: Arc<UI>) {
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
            *SHUTDOWN.write().await = true;
        }
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            *SHUTDOWN.write().await = true;
        }
        (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
            *RELOAD.write().await = true;
            *SHUTDOWN.write().await = true;
        }
        _ => {
            ui.handle_input(key).await;
        }
    }
}
