use chrono::Timelike;
use colored::Colorize;
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

    let hms = format!("{h:02}:{m:02}:{s:02}").dimmed();

    let name = match kind {
        Log::Info => "I".green(),
        Log::Warning => "W".yellow(),
        Log::Error => "E".red(),
    }
    .bold();

    println!("{name} {hms} {msg}");
}
