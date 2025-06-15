use chrono::Timelike;
use std::fmt::Display;

pub enum Log {
    Info,
    Warning,
    Error,
}

pub fn log(kind: Log, msg: impl Display) {
    let now = chrono::Local::now();
    let h = now.hour();
    let m = now.minute();
    let s = now.second();
    let hms = format!("{h:2}:{m:02}:{s:02}");

    let str = match kind {
        Log::Info => "INFO",
        Log::Warning => "WARNING",
        Log::Error => "ERROR",
    };

    println!("[{hms}] {str:7} {msg}");
}
