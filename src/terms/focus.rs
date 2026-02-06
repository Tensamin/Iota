#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Eula,
    Tos,
    Pp,
    Cancel,
    Continue,
    ContinueAll,
}

impl Focus {
    pub fn next(&mut self, state: (bool, bool, bool)) {
        *self = match self {
            Focus::Eula => Focus::Tos,
            Focus::Tos => Focus::Pp,
            Focus::Pp => Focus::Cancel,
            Focus::Cancel => {
                if state.0 {
                    Focus::Continue
                } else {
                    Focus::Eula
                }
            }
            Focus::Continue => {
                if state.0 && state.1 && state.2 {
                    Focus::ContinueAll
                } else {
                    Focus::Eula
                }
            }
            Focus::ContinueAll => Focus::Eula,
        };
    }

    pub fn prev(&mut self, state: (bool, bool, bool)) {
        *self = match self {
            Focus::Eula => {
                if state.0 && state.1 && state.2 {
                    Focus::ContinueAll
                } else if state.0 {
                    Focus::Continue
                } else {
                    Focus::Cancel
                }
            }
            Focus::Tos => Focus::Eula,
            Focus::Pp => Focus::Tos,
            Focus::Cancel => Focus::Pp,
            Focus::Continue => Focus::Cancel,
            Focus::ContinueAll => Focus::Continue,
        };
    }
}
