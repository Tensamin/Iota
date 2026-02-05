use crate::{
    terms::{doc::Doc, terms_checker, terms_getter::Type},
    util::file_util::{load_file, save_file},
};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ConsentManager;
impl ConsentManager {
    pub async fn check() -> (bool, bool) {
        let file = load_file("", "agreements");
        let existing = ConsentState::from_str(&file).sanitize();

        let final_state = if existing.accepted_eula {
            existing
        } else {
            let state = terms_checker::run_consent_ui(existing).await.sanitize();
            let string = state.clone().to_string().await;
            save_file("", "agreements", &string);
            state
        };

        (
            final_state.accepted_eula,
            final_state.accepted_pp && final_state.accepted_tos,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UserChoice {
    Deny,
    AcceptEULA,
    AcceptAll,
}

#[derive(Debug, Clone)]
pub struct ConsentState {
    pub eula: Option<Doc>,
    pub accepted_eula: bool,
    pub tos: Option<Doc>,
    pub accepted_tos: bool,
    pub pp: Option<Doc>,
    pub accepted_pp: bool,
}

impl ConsentState {
    fn sanitize(mut self) -> Self {
        if !self.accepted_eula {
            self.accepted_tos = false;
            self.accepted_pp = false;
        }
        self
    }

    async fn to_string(self) -> String {
        let current_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut file_out: String = format!(
            "This file reflects the current consent state used by the application.\
        \nIt may be regenerated or overwritten by the application.\
        \nThis file was last edited by Tensamin at:\
        \nUNIX-SECOND={}",
            current_secs
        );

        if self.accepted_eula
            && let Some(eula) = self.eula
        {
            file_out.push_str(&format!("\
            \n\"EULA=true\" indicates that you read, understood and accepted Tensamin's End User Licence agreement. You can find our EULA at https://legal.tensamin.net/eula/\
            \nEULA={}\
            \nEULA-VERSION={}\
            \nEULA-HASH={}\
            ", self.accepted_eula, eula.get_version(), eula.get_hash()));

            if self.accepted_tos
                && let Some(tos) = self.tos
            {
                file_out.push_str(&format!("\
                \n\"ToS=true\" indicates that you read, understood and accepted Tensamin's Terms of Service. You can find our Terms of Serivce at https://legal.tensamin.net/tos/\
                \nToS={}\
                \nToS-VERSION={}\
                \nToS-HASH={}\
                ", self.accepted_tos, tos.get_version(), tos.get_hash()));
            }
            if self.accepted_pp
                && let Some(pp) = self.pp
            {
                file_out.push_str(&format!("\
                \n\"Privacy-Policy=true\" indicates that you read, understood and accepted Tensamin's Privacy Policy. You can find our Privacy Policy at https://legal.tensamin.net/privacy/\
                \nPrivacy-Policy={}\
                \nPrivacy-Policy-VERSION={}\
                \nPrivacy-Policy-HASH={}\
                ", self.accepted_pp, pp.get_version(), pp.get_hash()));
            }
        } else {
            file_out.push_str("\
                \n\"EULA=true\" indicates that you read, understood and accepted Tensamin's End User Licence Agreement. You can find Tensamin's EULA at https://legal.tensamin.net/eula/\
                \nEULA=false\
                ");
        }
        file_out
    }

    fn from_str(s: &str) -> Self {
        let mut eula = false;
        let mut eula_version = String::new();
        let mut eula_hash = String::new();
        let mut pp = false;
        let mut pp_version = String::new();
        let mut pp_hash = String::new();
        let mut tos = false;
        let mut tos_version = String::new();
        let mut tos_hash = String::new();

        let mut unix = String::new();

        for line in s.lines() {
            if let Some(v) = line.strip_prefix("EULA=") {
                eula = v == "true";
            } else if let Some(v) = line.strip_prefix("EULA-VERSION=") {
                eula_version = v.to_string();
            } else if let Some(v) = line.strip_prefix("EULA-HASH=") {
                eula_hash = v.to_string();
            } else if let Some(v) = line.strip_prefix("ToS=") {
                tos = v == "true";
            } else if let Some(v) = line.strip_prefix("ToS-VERSION=") {
                tos_version = v.to_string();
            } else if let Some(v) = line.strip_prefix("ToS-HASH=") {
                tos_hash = v.to_string();
            } else if let Some(v) = line.strip_prefix("PrivacyPolicy=") {
                pp = v == "true";
            } else if let Some(v) = line.strip_prefix("PrivacyPolicy-VERSION=") {
                pp_version = v.to_string();
            } else if let Some(v) = line.strip_prefix("PrivacyPolicy-HASH=") {
                pp_hash = v.to_string();
            } else if let Some(v) = line.strip_prefix("UNIX=") {
                unix = v.to_string();
            }
        }
        let unix: u64 = unix.parse::<u64>().unwrap_or(0);
        Self {
            accepted_eula: eula,
            eula: Some(Doc::new(eula_version, eula_hash, Type::EULA, unix)),
            accepted_pp: pp,
            pp: Some(Doc::new(pp_version, pp_hash, Type::PP, unix)),
            accepted_tos: tos,
            tos: Some(Doc::new(tos_version, tos_hash, Type::TOS, unix)),
        }
        .sanitize()
    }
}
