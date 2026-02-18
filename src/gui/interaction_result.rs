use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;

use crate::gui::screens::screens::Screen;

pub enum InteractionResult {
    OpenScreen {
        screen: Box<dyn Screen>,
    },
    OpenFutureScreen {
        screen: Pin<Box<dyn Future<Output = Box<dyn Screen>> + Send>>,
    },
    Handeled,
    Unhandeled,
}

impl Debug for InteractionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InteractionResult::OpenScreen { screen: _ } => write!(f, "OpenScreen"),
            InteractionResult::OpenFutureScreen { screen: _ } => write!(f, "OpenFutureScreen"),
            InteractionResult::Handeled => write!(f, "Handeled"),
            InteractionResult::Unhandeled => write!(f, "Unhandeled"),
        }
    }
}
