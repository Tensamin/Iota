use crate::{
    terms::{
        doc::Doc,
        terms_checker,
        terms_getter::{Type, get_current_docs, get_newest_docs},
        terms_updater,
    },
    util::file_util::{load_file, save_file},
};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::task;

pub struct ConsentManager;
impl ConsentManager {
    pub async fn check() -> (bool, bool) {
        let file = load_file("", "agreements");
        let mut file_state = ConsentState::from_str(&file).sanitize();

        if !file_state.accepted_eula {
            file_state = terms_checker::run_consent_ui(file_state).await;

            println!("{}", &file_state.clone().sanitize().to_string());
            println!("-----------------------");

            save_file("", "agreements", &file_state.to_string());
        }

        if !file_state.accepted_eula {
            return (false, false);
        }

        if let Some((eula_update, tos_update, privacy_update)) = Self::get_updates().await {
            let is_forced = eula_update.is_err() || tos_update.is_err() || privacy_update.is_err();

            let updater_logic = async move {
                let state_to_update = terms_updater::run_consent_ui(
                    file_state,
                    eula_update,
                    tos_update,
                    privacy_update,
                )
                .await;

                println!("{}", &state_to_update.clone().sanitize().to_string());

                save_file("", "agreements", &state_to_update.sanitize().to_string());
            };

            if is_forced {
                updater_logic.await;
            } else {
                task::spawn(updater_logic);
            }
        }

        let final_state = ConsentState::from_str(&load_file("", "agreements")).sanitize();
        (
            final_state.accepted_eula,
            final_state.accepted_tos && final_state.accepted_pp,
        )
    }

    async fn get_updates() -> Option<(
        // Ok(None) indicates no update
        // Ok(Some) Indicates a future update
        // Err indicates a update that has to be accepted before the programm can continue
        Result<Option<Doc>, Doc>,
        Result<Option<Doc>, Doc>,
        Result<Option<Doc>, Doc>,
    )> {
        let file = load_file("", "agreements");
        let accepted_state = ConsentState::from_str(&file).sanitize();

        if let (
            Some((current_eula, current_tos, current_privacy)),
            Some((newest_eula, newest_tos, newest_privacy)),
        ) = (get_current_docs().await, get_newest_docs().await)
        {
            let eula_update: Result<Option<Doc>, Doc> =
                if current_eula.equals_some(&accepted_state.eula) {
                    if current_eula.equals(&newest_eula) {
                        Ok(None)
                    } else {
                        Ok(Some(newest_eula))
                    }
                } else if newest_eula.equals_some(&accepted_state.eula) {
                    Ok(None)
                } else {
                    Err(current_eula)
                };

            let tos_update: Result<Option<Doc>, Doc> = if newest_tos
                .equals_some(&accepted_state.tos)
            {
                Ok(None)
            } else if accepted_state.accepted_tos && current_tos.equals_some(&accepted_state.tos) {
                if current_tos.equals(&newest_tos) {
                    Ok(None)
                } else {
                    Ok(Some(newest_tos))
                }
            } else {
                Err(current_tos)
            };

            let privacy_update: Result<Option<Doc>, Doc> =
                if newest_privacy.equals_some(&accepted_state.privacy) {
                    Ok(None)
                } else if accepted_state.accepted_pp
                    && current_privacy.equals_some(&accepted_state.privacy)
                {
                    if current_privacy.equals(&newest_privacy) {
                        Ok(None)
                    } else {
                        Ok(Some(newest_privacy))
                    }
                } else {
                    Err(current_privacy)
                };
            Some((eula_update, tos_update, privacy_update))
        } else {
            None
        }
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
    pub privacy: Option<Doc>,
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

    pub fn to_string(&self) -> String {
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

        if let Some(eula) = &self.eula {
            println!(
                "EULA: {}, {}, {}",
                self.accepted_eula,
                eula.get_version(),
                eula.get_hash()
            );
            file_out.push_str(&format!("\
            \n\"EULA=true\" indicates that you read, understood and accepted Tensamin's End User Licence agreement. You can find our EULA at https://legal.tensamin.net/eula/\
            \nEULA={}\
            \nEULA-VERSION={}\
            \nEULA-HASH={}\
            ", self.accepted_eula, eula.get_version(), eula.get_hash()));

            if self.accepted_tos
                && let Some(tos) = &self.tos
            {
                file_out.push_str(&format!("\
                \n\"Terms-of-Service=true\" indicates that you read, understood and accepted Tensamin's Terms of Service. You can find our Terms of Serivce at https://legal.tensamin.net/tos/\
                \nTerms-of-Service={}\
                \nTerms-of-Service-VERSION={}\
                \nTerms-of-Service-HASH={}\
                ", self.accepted_tos, tos.get_version(), tos.get_hash()));
            }
            if self.accepted_pp
                && let Some(pp) = &self.privacy
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
            } else if let Some(v) = line.strip_prefix("Terms-of-Service=") {
                tos = v == "true";
            } else if let Some(v) = line.strip_prefix("Terms-of-Service-VERSION=") {
                tos_version = v.to_string();
            } else if let Some(v) = line.strip_prefix("Terms-of-Service-HASH=") {
                tos_hash = v.to_string();
            } else if let Some(v) = line.strip_prefix("Privacy-Policy=") {
                pp = v == "true";
            } else if let Some(v) = line.strip_prefix("Privacy-Policy-VERSION=") {
                pp_version = v.to_string();
            } else if let Some(v) = line.strip_prefix("Privacy-Policy-HASH=") {
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
            privacy: Some(Doc::new(pp_version, pp_hash, Type::PP, unix)),
            accepted_tos: tos,
            tos: Some(Doc::new(tos_version, tos_hash, Type::TOS, unix)),
        }
        .sanitize()
    }
}
