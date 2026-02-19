use crate::gui::{
    elements::{
        console_card::ConsoleCard,
        elements::{Element, InteractableElement, JoinableElement},
        log_card::LogCard,
    },
    interaction_result::InteractionResult,
    screens::screens::{NavDirection, Screen},
    ui::UI,
};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    widgets::{Block, Borders},
};

use std::{any::Any, sync::Arc};

pub struct MainScreen {
    ui: Arc<UI>,
    elements: Vec<Box<dyn InteractableElement>>,
    nav_grid: Vec<Vec<Option<usize>>>,
    selected_coords: (usize, usize),
}

impl MainScreen {
    pub async fn new(ui: Arc<UI>) -> Self {
        let mut elements: Vec<Box<dyn InteractableElement>> = Vec::new();
        let mut nav_grid: Vec<Vec<Option<usize>>> = Vec::new();

        let mut log_card = LogCard::new();
        log_card.set_borders(Borders::TOP.union(Borders::RIGHT).union(Borders::LEFT));
        let mut console_card = ConsoleCard::new("Console", "");
        console_card.set_joins(Borders::TOP);
        elements.push(Box::new(log_card));
        elements.push(Box::new(console_card));

        nav_grid.push(vec![Some(0)]);
        nav_grid.push(vec![Some(1)]);

        let mut screen = MainScreen {
            ui,
            elements,
            nav_grid,
            selected_coords: (1, 0),
        };

        screen.focus_current();
        screen
    }

    fn focus_current(&mut self) {
        let (y, x) = self.selected_coords;
        if let Some(Some(index)) = self.nav_grid.get(y).and_then(|row| row.get(x)) {
            if let Some(element) = self.elements.get_mut(*index) {
                if element.can_focus() {
                    element.focus(true);
                }
            }
        }
    }

    fn unfocus_current(&mut self, y: usize, x: usize) {
        if let Some(Some(index)) = self.nav_grid.get(y).and_then(|row| row.get(x)) {
            if let Some(element) = self.elements.get_mut(*index) {
                element.focus(false);
            }
        }
    }

    fn navigate(&mut self, direction: NavDirection) {
        let (mut y, mut x) = self.selected_coords;

        self.unfocus_current(y, x);

        match direction {
            NavDirection::Up => {
                if y > 0 {
                    y -= 1;
                }
            }
            NavDirection::Down => {
                if y < self.nav_grid.len() - 1 {
                    y += 1;
                }
            }
            NavDirection::Left => {
                if x > 0 {
                    x -= 1;
                }
            }
            NavDirection::Right => {
                if let Some(row) = self.nav_grid.get(y) {
                    if x < row.len() - 1 {
                        x += 1;
                    }
                }
            }
            _ => {}
        }

        // Clamp X to row length
        if let Some(row) = self.nav_grid.get(y) {
            if x >= row.len() {
                x = row.len() - 1;
            }
        }

        if self
            .nav_grid
            .get(y)
            .and_then(|r| r.get(x))
            .map_or(false, |e| e.is_some())
        {
            self.selected_coords = (y, x);
            self.focus_current();
        }
    }
}

impl Screen for MainScreen {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_ui(&self) -> &Arc<UI> {
        &self.ui
    }

    fn render(&self, f: &mut Frame, rect: Rect) {
        let main_block = Block::default().title("Main").borders(Borders::ALL);
        f.render_widget(main_block, rect);

        let inner = rect.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).split(inner);

        if let Some(Some(index)) = self.nav_grid.get(0).and_then(|r| r.get(0)) {
            self.elements[*index].as_element().render(f, chunks[0]);
        }

        if let Some(Some(index)) = self.nav_grid.get(1).and_then(|r| r.get(0)) {
            self.elements[*index].as_element().render(f, chunks[1]);
        }
    }

    fn handle_input(&mut self, event: KeyEvent) -> InteractionResult {
        match event.code {
            KeyCode::Up => self.navigate(NavDirection::Up),
            KeyCode::Down => self.navigate(NavDirection::Down),
            KeyCode::Left => self.navigate(NavDirection::Left),
            KeyCode::Right => self.navigate(NavDirection::Right),
            _ => {
                let (y, x) = self.selected_coords;
                if let Some(Some(index)) = self.nav_grid.get(y).and_then(|r| r.get(x)) {
                    if let Some(el) = self.elements.get_mut(*index) {
                        return el.interact(event);
                    }
                }
            }
        }

        InteractionResult::Handled
    }
}
