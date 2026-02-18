use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::pin::Pin;

use crate::gui::screens::screens::Screen;

#[allow(unused)]
pub enum InteractionResult {
    CloseScreen,
    OpenScreen {
        screen: Box<dyn Screen>,
    },
    OpenFutureScreen {
        screen: Pin<Box<dyn Future<Output = Box<dyn Screen>> + Send>>,
    },
    Handled,
    Unhandled,
}

impl Debug for InteractionResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InteractionResult::OpenScreen { screen: _ } => write!(f, "OpenScreen"),
            InteractionResult::OpenFutureScreen { screen: _ } => write!(f, "OpenFutureScreen"),
            InteractionResult::CloseScreen => write!(f, "CloseScreen"),
            InteractionResult::Handled => write!(f, "Handled"),
            InteractionResult::Unhandled => write!(f, "Unhandled"),
        }
    }
}
