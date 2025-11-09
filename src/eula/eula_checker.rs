use crate::util::file_util::{load_file, save_file};

pub fn check_eula() -> bool {
    let eula = "By changing the value to \"true\" you agree to our end user license agreement and our terms of service!\
    \nYou can find our Terms of service on https://docs.tensamin.net/legal/terms-of-service/.\
    \neula=false";
    let file = load_file("", "eula.txt");
    if file.is_empty() {
        save_file("", "eula.txt", eula);
        return false;
    }

    if (file.contains("eula=false")) {
        false
    } else if (file.contains("eula=true")) {
        true
    } else {
        false
    }
}
pub fn accept_eula() {
    let eula = "By changing the value to \"true\" you agree to our end user license agreement and our terms of service!\
    \nYou can find our Terms of service on https://docs.tensamin.net/legal/terms-of-service/.\
    \neula=true";
    save_file("", "eula.txt", eula);
}
