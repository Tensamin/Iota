use crate::{
    terms::{terms_checker::run_consent_ui, terms_getter::get_current_docs},
    util::file_util::{load_file, save_file},
};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ConsentManager;
impl ConsentManager {
    pub async fn check() -> (bool, bool) {
        let file = load_file("", "agreements");
        let existing = ConsentUiState::from_str(&file).sanitize();

        let final_state = if existing.eula {
            existing
        } else {
            let choice = run_consent_ui().await;
            let state = match choice {
                UserChoice::Deny => ConsentUiState::denied(),
                UserChoice::AcceptEULA => ConsentUiState {
                    eula: true,
                    tos: false,
                    pp: false,
                },
                UserChoice::AcceptAll => ConsentUiState {
                    eula: true,
                    tos: true,
                    pp: true,
                },
            };
            let state = state.sanitize();
            if let Ok(string) = state.to_string().await {
                save_file("", "agreements", &string);
            }
            state
        };

        (final_state.eula, final_state.pp && final_state.tos)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UserChoice {
    Deny,
    AcceptEULA,
    AcceptAll,
}

#[derive(Debug, Clone, Copy)]
pub struct ConsentUiState {
    pub eula: bool,
    pub tos: bool,
    pub pp: bool,
}

impl ConsentUiState {
    fn denied() -> Self {
        Self {
            eula: false,
            pp: false,
            tos: false,
        }
    }

    fn sanitize(mut self) -> Self {
        if !self.eula {
            self.tos = false;
            self.pp = false;
        }
        self
    }

    async fn to_string(self) -> Result<String, ()> {
        let current_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Some((eula, tos, pp)) = get_current_docs().await {
            Ok(format!(
                "\
            \"EULA=true\" indicates that you read, understood and accepted the End User Licence agreement. You can find our EULA at https://legal.tensamin.net/eula/\
            \nEULA={}\
            \nEULA-VERSION={}\
            \nEULA-HASH={}\
            \n\"PrivacyPolicy=true\" indicates that you read, understood and accepted the Privacy Policy. You can find our Privacy Policy at https://legal.tensamin.net/privacy-policy/\
            \nPrivacyPolicy={}\
            \nPrivacyPolicy-VERSION={}\
            \nPrivacyPolicy-HASH={}\
            \n\"ToS=true\" indicates that you read, understood and accepted the Terms of Service. You can find our Terms of Service at https://legal.tensamin.net/terms-of-service/\
            \nToS={}\
            \nToS-VERSION={}\
            \nToS-HASH={}\
            \nThis file reflects the current consent state used by the application.\
            \nIt may be regenerated or overwritten by the application.\
            \nThis file was last edited by Tensamin at:\
            \nUNIX-SECOND={}\
            ",
                self.eula,
                eula.get_version(),
                eula.get_hash(),
                self.tos,
                tos.get_version(),
                tos.get_hash(),
                self.pp,
                pp.get_version(),
                pp.get_hash(),
                current_secs
            ))
        } else {
            Err(())
        }
    }

    fn from_str(s: &str) -> Self {
        let mut eula = false;
        let mut pp = false;
        let mut tos = false;

        for line in s.lines() {
            if let Some(v) = line.strip_prefix("EULA=") {
                eula = v == "true";
            } else if let Some(v) = line.strip_prefix("ToS=") {
                tos = v == "true";
            } else if let Some(v) = line.strip_prefix("PrivacyPolicy=") {
                pp = v == "true";
            }
        }

        Self { eula, pp, tos }.sanitize()
    }
    pub fn can_continue(&self) -> bool {
        self.eula
    }
    pub fn can_continue_all(&self) -> bool {
        self.eula && self.pp && self.tos
    }
}
