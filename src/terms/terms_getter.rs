#[derive(Clone)]
pub enum Type {
    EULA,
    TOS,
    PP,
}

impl Type {
    pub fn to_str(&self) -> &str {
        match self {
            Self::EULA => "eula",
            Self::TOS => "tos",
            Self::PP => "pp",
        }
    }
    pub fn to_string(&self) -> String {
        match self {
            Self::EULA => "End User License Agreement".to_string(),
            Self::TOS => "Terms of Service".to_string(),
            Self::PP => "Privacy Policy".to_string(),
        }
    }
}
pub fn get_link(terms_type: Type) -> String {
    format!("https://legal.tensamin.net/{}/", terms_type.to_str())
}
pub async fn get_terms(terms_type: Type) -> Option<String> {
    let body = reqwest::get(format!(
        "https://legal.tensamin.net/api/text/{}/",
        terms_type.to_str()
    ))
    .await
    .ok()?
    .text()
    .await
    .ok()?;

    Some(body)
}
