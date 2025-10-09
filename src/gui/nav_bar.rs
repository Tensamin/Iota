use std::str;

use crate::gui::log_panel;

pub struct Screen {
    pub title: String,
    pub render: Box<dyn Fn() + Send + Sync + 'static>,
}
impl Screen {
    pub fn new(title: &str, render: Box<dyn Fn() + Send + Sync + 'static>) -> Self {
        Screen {
            title: String::from(title),
            render,
        }
    }
    pub fn renderf(&self) {
        (self.render)();
    }
}
pub struct NavBar {
    pub current_screen: Screen,
    pub screens: Vec<Screen>,
}
impl NavBar {
    pub fn new() -> Self {
        NavBar {
            current_screen: Screen {
                title: String::from("Main"),
                render: Box::new(|| log_panel::render()),
            },
            screens: Vec::new(),
        }
    }
}
