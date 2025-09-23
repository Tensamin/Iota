use crate::util::file_util::{self, save_file};
use json::{self, JsonValue};

pub fn create_languages() {
    let mut frontend_messages = JsonValue::new_object();
    let mut omikron_messages = JsonValue::new_object();
    let mut button_texts = JsonValue::new_object();
    let mut general_texts = JsonValue::new_object();
    let mut debug_messages = JsonValue::new_object();

    // FRONTEND
    frontend_messages.insert(
        "USER_CONTEXT_GET_CONVERSATIONS",
        "User {} is loading conversations",
    );
    frontend_messages.insert(
        "USER_CONTEXT_GET_COMMUNITIES",
        "User {} is loading communities",
    );
    frontend_messages.insert("ADD_CONVERSATION", "User {} added {}");

    // OMIKRON
    omikron_messages.insert(
        "IdentificationResponse",
        "IOTA identified on Omikron, {} users!",
    );

    // BUTTONS
    button_texts.insert("EXIT", "Exit");

    // GENERAL
    general_texts.insert("IOTA_ID", "IOTA ID: {}-####-####-####-############");
    general_texts.insert("USER_ID", "USER ID: {}");
    general_texts.insert("USER_IDS", "USER IDS: {}");

    // DEBUG
    debug_messages.insert("", "");
    save_file(
        "languages/en_INT",
        "frontend.json",
        &frontend_messages.to_string(),
    );
    save_file(
        "languages/en_INT",
        "omikron.json",
        &omikron_messages.to_string(),
    );
    save_file(
        "languages/en_INT",
        "buttons.json",
        &button_texts.to_string(),
    );
    save_file(
        "languages/en_INT",
        "debug.json",
        &debug_messages.to_string(),
    );
}
