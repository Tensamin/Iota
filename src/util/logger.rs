use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    sync::{OnceLock, mpsc},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use ratatui::style::Color;

use crate::{
    APP_STATE,
    gui::{elements::log_card::UiLogEntry, ui::UNIQUE},
    langu::language_manager,
};

static LOGGER: OnceLock<mpsc::Sender<LogMessage>> = OnceLock::new();

#[derive(Clone, Copy)]
#[allow(unused)]
pub enum PrintType {
    Call,
    Client,
    Iota,
    Omikron,
    Omega,
    General,
}

struct LogMessage {
    timestamp_ms: u128,
    prefix: String,
    kind: PrintType,
    is_error: bool,

    translation_key: Option<String>,
    format_args: Vec<String>,

    message: Option<String>,
}

pub fn startup() {
    let (tx, rx) = mpsc::channel::<LogMessage>();
    LOGGER.set(tx).expect("Logger already initialized");

    thread::spawn(move || {
        let log_dir = Path::new("logs");
        fs::create_dir_all(log_dir).expect("Failed to create log directory");

        let start_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let path = log_dir.join(format!("log_{}.txt", start_ts));
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("Failed to open log file");

        for msg in rx {
            let resolved_message = if let Some(key) = msg.translation_key {
                let args: Vec<&str> = msg.format_args.iter().map(|s| s.as_str()).collect();
                language_manager::format(&key, &args)
            } else {
                msg.message.unwrap_or_default()
            };

            let ts = fixed_box(&msg.timestamp_ms.to_string(), 13);

            let line = format!("{} {}", msg.prefix, resolved_message);

            let _ = writeln!(file, "{} {}", ts, line);

            let color = colorize(msg.kind, msg.is_error);

            let ui_entry = UiLogEntry {
                line: line.clone(),
                color,
            };

            {
                let mut state = APP_STATE.lock().unwrap();
                state.push_log(ui_entry);
            }
        }
    });
}

fn colorize(kind: PrintType, is_error: bool) -> Color {
    if is_error {
        return Color::Red;
    }

    match kind {
        PrintType::Call => Color::Magenta,
        PrintType::Client => Color::Green,
        PrintType::Iota => Color::Yellow,
        PrintType::Omikron => Color::Blue,
        PrintType::Omega => Color::Cyan,
        PrintType::General => Color::White,
    }
}

fn fixed_box(content: &str, width: usize) -> String {
    let s: String = content.chars().take(width).collect();
    let len = s.chars().count();
    if len < width {
        format!("[{}{}]", " ".repeat(width - len), s)
    } else {
        s
    }
}

pub fn log_internal_translated(
    kind: PrintType,
    prefix: String,
    is_error: bool,
    key: &str,
    args: Vec<String>,
) {
    if let Some(tx) = LOGGER.get() {
        tokio::spawn(async move {
            *UNIQUE.write().await = true;
        });
        let _ = tx.send(LogMessage {
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            prefix,
            kind,
            is_error,
            translation_key: Some(key.to_string()),
            format_args: args,
            message: None,
        });
    }
}

pub fn log_internal(kind: PrintType, prefix: String, is_error: bool, message: String) {
    if let Some(tx) = LOGGER.get() {
        tokio::spawn(async move {
            *UNIQUE.write().await = true;
        });
        let _ = tx.send(LogMessage {
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            prefix,
            kind,
            is_error,
            translation_key: None,
            format_args: Vec::new(),
            message: Some(message),
        });
    }
}

use crate::data::communication::CommunicationValue;
use json::JsonValue;
pub fn log_cv_internal(prefix: String, cv: &CommunicationValue, print_type: Option<PrintType>) {
    let formatted = format_cv(cv);

    log_internal(
        print_type.unwrap_or(PrintType::General),
        prefix,
        false,
        formatted,
    );
}

pub fn format_cv(cv: &CommunicationValue) -> String {
    let mut parts = Vec::new();

    let sender = cv.get_sender();
    let receiver = cv.get_receiver();

    if sender > 0 && receiver > 0 {
        parts.push(format!("{} > {}", sender, receiver));
    } else if sender > 0 {
        parts.push(format!("{}", sender));
    } else if receiver > 0 {
        parts.push(format!("> {}", receiver));
    }

    let comm_type = cv.get_type().to_string();
    parts.push(format!("{}", comm_type));

    let mut data_parts = Vec::new();
    if let JsonValue::Object(data) = &cv.clone().to_json()["data"] {
        for (key, value) in data.iter() {
            let val_string = match value {
                JsonValue::String(s) => s.clone(),
                _ => value.dump(),
            };

            data_parts.push(format!("{} {}", key, val_string));
        }
    }

    if !data_parts.is_empty() {
        parts.push(format!("{}", data_parts.join(", ")));
    }

    parts.join(": ")
}
#[macro_export]
macro_rules! log_cv {
    ($kind:expr, $cv:expr) => {
        $crate::util::logger::log_cv_internal("".to_string(), &$cv, Some($kind))
    };
    ($cv:expr) => {
        $crate::util::logger::log_cv_internal("".to_string(), &$cv, None)
    };
}
#[macro_export]
macro_rules! log_cv_in {
    ($kind:expr, $cv:expr) => {
        $crate::util::logger::log_cv_internal("> ".to_string(), &$cv, Some($kind))
    };
    ($cv:expr) => {
        $crate::util::logger::log_cv_internal("> ".to_string(), &$cv, None)
    };
}
#[macro_export]
macro_rules! log_cv_out {
    ($kind:expr, $cv:expr) => {
        $crate::util::logger::log_cv_internal("< ".to_string(), &$cv, Some($kind))
    };
    ($cv:expr) => {
        $crate::util::logger::log_cv_internal("< ".to_string(), &$cv, None)
    };
}

#[macro_export]
macro_rules! log_t {
    ($key:expr) => {
        $crate::util::logger::log_internal_translated(
            $crate::util::logger::PrintType::General,
            "".to_string(),
            false,
            $key,
            vec![]
        )
    };

    ($key:expr, $($arg:expr),+) => {
        $crate::util::logger::log_internal_translated(
            $crate::util::logger::PrintType::General,
            "".to_string(),
            false,
            $key,
            vec![$($arg),+]
        )
    };
}
#[macro_export]
macro_rules! log_t_err {
    ($key:expr) => {
        $crate::util::logger::log_internal_translated(
            $crate::util::logger::PrintType::General,
            "".to_string(),
            true,
            $key,
            vec![]
        )
    };

    ($key:expr, $($arg:expr),+) => {
        $crate::util::logger::log_internal_translated(
            $crate::util::logger::PrintType::General,
            "".to_string(),
            true,
            $key,
            vec![$($arg.to_string()),+]
        )
    };
}

/// Log a general informational message.
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::util::logger::log_internal($crate::util::logger::PrintType::General, "".to_string(), false, format!($($arg)*))
    };
}
/// Log an inbound message (`>`).
#[macro_export]
macro_rules! log_in {
    ($($arg:tt)*) => {
        $crate::util::logger::log_internal(
            $crate::util::logger::PrintType::General,
            ">".to_string(),
            false,
            format!($($arg)*)
        )
    };
}
/// Log an outbound message (`<`).
#[macro_export]
macro_rules! log_out {
    ($($arg:tt)*) => {
        $crate::util::logger::log_internal(
            $crate::util::logger::PrintType::General,
            "<".to_string(),
            false,
            format!($($arg)*)
        )
    };
}
/// Log an error message (`>>`).
#[macro_export]
macro_rules! log_err {
    ($($arg:tt)*) => {
        $crate::util::logger::log_internal(
            $crate::util::logger::PrintType::General,
            ">>".to_string(),
            true,
            format!($($arg)*)
        )
    };
}
