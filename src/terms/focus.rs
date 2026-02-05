use crate::terms::consent_state::ConsentState;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Eula,
    Tos,
    Pp,
    Cancel,
    Continue,
    ContinueAll,
}
impl Focus {
    pub fn next(&mut self, state: ConsentState) {
        *self = match self {
            Focus::Eula => Focus::Tos,
            Focus::Tos => Focus::Pp,
            Focus::Pp => Focus::Cancel,
            Focus::Cancel => {
                if state.can_continue() {
                    Focus::Continue
                } else {
                    Focus::Eula
                }
            }
            Focus::Continue => {
                if state.can_continue_all() {
                    Focus::ContinueAll
                } else {
                    Focus::Eula
                }
            }
            Focus::ContinueAll => Focus::Eula,
        };
    }

    pub fn prev(&mut self, state: ConsentState) {
        *self = match self {
            Focus::Eula => {
                if state.can_continue_all() {
                    Focus::ContinueAll
                } else if state.can_continue() {
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
