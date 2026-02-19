use crossterm::event::{KeyCode, KeyEvent};
use json::JsonValue;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    ACTIVE_TASKS,
    data::communication::{CommunicationType, CommunicationValue, DataTypes},
    gui::{
        elements::elements::{Element, InteractableElement, JoinableElement},
        interaction_result::InteractionResult,
        ui::{FPS, UI},
        util::borders::draw_block_joins,
    },
    log, log_cv,
    omikron::omikron_connection::OMIKRON_CONNECTION,
    users::{user_manager, user_profile::UserProfile},
    util::file_util,
};
use std::{
    any::Any,
    sync::Arc,
    time::{Duration, SystemTime},
};

pub struct ConsoleCard {
    focused: bool,
    pub title: String,
    pub content: String,

    borders: Borders,
    joins: Borders,
}

impl ConsoleCard {
    pub fn new(title: &str, content: &str) -> Self {
        ConsoleCard {
            focused: false,
            title: title.to_string(),
            content: content.to_string(),
            borders: Borders::ALL,
            joins: Borders::NONE,
        }
    }
}

impl Element for ConsoleCard {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn render(&self, f: &mut Frame, r: Rect) {
        let block = Block::default()
            .borders(self.borders)
            .title(self.title.clone())
            .title_style(Style::default().fg(Color::White))
            .border_style(if self.focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .style(if self.focused {
                Style::default().fg(Color::White)
            } else {
                Style::default()
            });

        let par = Paragraph::new(Line::from(Span::from(self.content.clone())))
            .block(block)
            .scroll((0, 0));
        f.render_widget(par, r);
        draw_block_joins(f, r, self.borders, self.joins);
    }
}

impl JoinableElement for ConsoleCard {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_element(&self) -> &dyn Element {
        self
    }

    fn as_element_mut(&mut self) -> &mut dyn Element {
        self
    }

    fn set_borders(&mut self, borders: Borders) {
        self.borders = borders;
    }

    fn set_joins(&mut self, joins: Borders) {
        self.joins = joins;
    }
}

impl InteractableElement for ConsoleCard {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_element(&self) -> &dyn Element {
        self
    }

    fn as_element_mut(&mut self) -> &mut dyn Element {
        self
    }

    fn interact(&mut self, key: KeyEvent) -> InteractionResult {
        match key.code {
            KeyCode::Enter => {
                let command = self.content.clone();
                tokio::spawn(async move {
                    run_command(&command).await;
                });
                self.content = "".to_string();
                InteractionResult::Handled
            }
            KeyCode::Backspace => {
                self.content.pop();
                InteractionResult::Handled
            }
            _ => {
                if let Some(c) = key.code.as_char() {
                    self.content.push(c);
                    InteractionResult::Handled
                } else {
                    InteractionResult::Unhandled
                }
            }
        }
    }

    fn can_focus(&self) -> bool {
        true
    }

    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self, f: bool) {
        self.focused = f;
    }
}
pub async fn run_command(command: &str) {
    log!(":{}", command);

    let parts = command.split(" ").collect::<Vec<&str>>();

    match parts.as_slice() {
        ["tasks"] => {
            let active_tasks: Vec<String> =
                ACTIVE_TASKS.clone().iter().map(|v| v.to_string()).collect();
            log!("Active tasks: {:?}", active_tasks);
        }
        ["fps"] => {
            log!("FPS: {}", *FPS.read().await);
        }
        ["ping"] => {
            let time = 20;

            let now = SystemTime::now();

            let conn = {
                let guard = OMIKRON_CONNECTION.read().await;
                guard.as_ref().cloned()
            };

            let conn = match conn {
                Some(c) => c,
                None => return,
            };

            let response_cv = conn
                .await_response(
                    &CommunicationValue::new(CommunicationType::ping),
                    Some(Duration::from_secs(time)),
                )
                .await;

            let elapsed = now.elapsed().unwrap_or(Duration::ZERO);

            match response_cv {
                Ok(response) => log_cv!(response.add_data(
                    DataTypes::get_time,
                    JsonValue::from(elapsed.as_millis() as i64)
                )),
                Err(err) => log!("Ping error: {:?}", err),
            }
        }
        ["ping", time] => {
            let time = time.parse::<u64>().unwrap_or(20);

            let conn = {
                let guard = OMIKRON_CONNECTION.read().await;
                guard.as_ref().cloned()
            };

            let conn = match conn {
                Some(c) => c,
                None => return,
            };

            let response_cv = conn
                .await_response(
                    &CommunicationValue::new(CommunicationType::ping),
                    Some(Duration::from_secs(time)),
                )
                .await;
            match response_cv {
                Ok(response) => log_cv!(response),
                Err(err) => log!("Ping error: {:?}", err),
            }
        }
        ["user", "add", username] => {
            if let (Some(user), Some(_)) = user_manager::create_user(username).await {
                log!("Created user {}", user.user_id);
            } else {
                log!("Failed to create user");
            }
        }
        ["user", "remove", username] => {
            if let Some(user) = user_manager::get_user_by_username(username) {
                user_manager::remove_user(user.user_id);
                log!("Removed user {}", user.user_id);
            } else {
                log!("Failed to find user");
            }
        }
        ["user", "list"] => {
            let users: Vec<UserProfile> = user_manager::get_users();
            for user in users {
                let storage = file_util::get_designed_storage(user.user_id);
                log!(
                    "> Username: {}, ID: {}, created at: {}, storage: {}",
                    user.username,
                    user.user_id,
                    user.created_at,
                    storage
                );
            }
        }
        ["user", "info", username] => {
            if let Some(user) = user_manager::get_user_by_username(username) {
                user_manager::remove_user(user.user_id);
                log!("Removed user {}", user.user_id);
            } else {
                log!("Failed to find user");
            }
        }
        _ => {
            log!("Unknown command");
        }
    }
}
