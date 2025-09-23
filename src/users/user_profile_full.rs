use crate::users::user_profile::UserProfile;

// --- UserProfileFull ---
#[derive(Clone, Debug)]
pub struct UserProfileFull {
    pub user_profile: UserProfile,
    pub private_key: String,
}
