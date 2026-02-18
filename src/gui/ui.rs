use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType, EnterAlternateScreen},
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    layout::Size,
};
use std::{
    io::{self, Error},
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::gui::{interaction_result::InteractionResult, screens::screens::Screen};

struct ResizableBackend<'a> {
    backend: CrosstermBackend<&'a mut Vec<u8>>,
    size: Size,
}

impl<'a> ResizableBackend<'a> {
    fn new(buf: &'a mut Vec<u8>, width: u16, height: u16) -> Self {
        Self {
            backend: CrosstermBackend::new(buf),
            size: Size::new(width, height),
        }
    }
}

impl<'a> Backend for ResizableBackend<'a> {
    type Error = Error;

    fn draw<'b, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'b ratatui::buffer::Cell)>,
    {
        self.backend.draw(content)
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.backend.hide_cursor()
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.backend.show_cursor()
    }

    #[allow(deprecated)]
    fn get_cursor(&mut self) -> io::Result<(u16, u16)> {
        self.backend.get_cursor()
    }

    #[allow(deprecated)]
    fn set_cursor(&mut self, x: u16, y: u16) -> io::Result<()> {
        self.backend.set_cursor(x, y)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.backend.clear()
    }

    fn clear_region(&mut self, region: ratatui::backend::ClearType) -> io::Result<()> {
        self.backend.clear_region(region)
    }

    fn size(&self) -> Result<Size, Error> {
        Ok(self.size)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.backend.flush()
    }

    fn get_cursor_position(&mut self) -> Result<ratatui::prelude::Position, Self::Error> {
        todo!()
    }

    fn set_cursor_position<P: Into<ratatui::prelude::Position>>(
        &mut self,
        _position: P,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn window_size(&mut self) -> Result<ratatui::prelude::backend::WindowSize, Self::Error> {
        todo!()
    }
}

/// UI state and rendering

pub struct UI {
    pub cols: Arc<Mutex<u16>>,
    pub rows: Arc<Mutex<u16>>,
    screen: Arc<Mutex<Option<Box<dyn Screen>>>>,

    cached_render: Arc<Mutex<Vec<u8>>>,
}

impl UI {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols: Arc::new(Mutex::new(cols)),
            rows: Arc::new(Mutex::new(rows)),
            screen: Arc::new(Mutex::new(None)),
            cached_render: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn set_screen(&self, screen: Box<dyn Screen>) {
        *self.screen.lock().unwrap() = Some(screen);
    }

    pub fn resize(&self, cols: u32, rows: u32) {
        *self.cols.lock().unwrap() = cols as u16;
        *self.rows.lock().unwrap() = rows as u16;
    }

    pub async fn handle_input(&self, input: &[u8]) {
        let key_event = if input.len() == 1 {
            let c = input[0] as char;
            if c.is_ascii() {
                Some(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
            } else {
                None
            }
        } else {
            None
        };

        let event = match input {
            b"\x1b[A" => Some(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
            b"\x1b[B" => Some(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
            b"\x1b[C" => Some(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
            b"\x1b[D" => Some(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
            b"\r" | b"\n" => Some(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            b"\x7f" => Some(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
            _ => key_event,
        };

        if let Some(event) = event {
            let result = {
                let mut guard = self.screen.lock().unwrap();
                if let Some(screen) = guard.as_mut() {
                    screen.handle_input(event)
                } else {
                    return;
                }
            };
            match result {
                InteractionResult::OpenScreen { screen } => {
                    self.set_screen(screen);
                }
                InteractionResult::OpenFutureScreen { screen: fut } => {
                    let ui = self.clone();
                    let screen = fut.await;
                    ui.set_screen(screen);
                }
                _ => {}
            }
        }
    }

    pub fn render(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        execute!(
            &mut buf,
            EnterAlternateScreen,
            Clear(ClearType::All),
            MoveTo(0, 0)
        )
        .unwrap();

        {
            let backend = ResizableBackend::new(
                &mut buf,
                *self.cols.lock().unwrap(),
                *self.rows.lock().unwrap(),
            );

            let mut terminal = Terminal::new(backend).unwrap();

            if let Some(screen) = self.screen.lock().unwrap().as_ref() {
                let _ = terminal.draw(|f| screen.render(f, f.area()));
            }
        }

        *self.cached_render.lock().unwrap() = buf.clone();

        buf
    }
}
