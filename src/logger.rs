use colored::Colorize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static DEBUG_MODE: AtomicBool = AtomicBool::new(false);

pub fn set_debug(enabled: bool) {
    DEBUG_MODE.store(enabled, Ordering::Relaxed);
}

pub fn is_debug() -> bool {
    DEBUG_MODE.load(Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Success,
    Warn,
    Error,
    Debug,
}

fn timestamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn log(level: LogLevel, message: &str) {
    if level == LogLevel::Debug && !is_debug() {
        return;
    }

    crate::console::push(
        "internal",
        match level {
            LogLevel::Info => "info",
            LogLevel::Success => "ok",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
            LogLevel::Debug => "debug",
        },
        message,
    );

    let ts = timestamp();
    let prefix = match level {
        LogLevel::Info => "[INFO]".cyan().bold(),
        LogLevel::Success => "[OK]".green().bold(),
        LogLevel::Warn => "[WARN]".yellow().bold(),
        LogLevel::Error => "[ERROR]".red().bold(),
        LogLevel::Debug => "[DEBUG]".magenta().bold(),
    };

    println!("{} {} {}", ts.dimmed(), prefix, message);
}

pub fn info(message: &str) {
    log(LogLevel::Info, message);
}

pub fn success(message: &str) {
    log(LogLevel::Success, message);
}

pub fn warn(message: &str) {
    log(LogLevel::Warn, message);
}

pub fn error(message: &str) {
    log(LogLevel::Error, message);
}

pub fn debug(message: &str) {
    log(LogLevel::Debug, message);
}

pub fn http_request(method: &str, path: &str, status: u16, duration: Duration) {
    crate::console::push(
        "web",
        if status < 400 { "ok" } else if status < 500 { "warn" } else { "error" },
        format!("{} {} -> {} ({} ms)", method, path, status, duration.as_millis()),
    );

    let debug = is_debug();

    if !debug && status < 400 {
        return;
    }

    let ts = timestamp();
    let prefix = "[HTTP]".blue().bold();
    let status_styled = if status < 300 {
        format!("{}", status).green()
    } else if status < 400 {
        format!("{}", status).cyan()
    } else if status < 500 {
        format!("{}", status).yellow()
    } else {
        format!("{}", status).red().bold()
    };

    let ms = duration.as_millis();
    let dur_styled = if ms < 50 {
        format!("{}ms", ms).green()
    } else if ms < 200 {
        format!("{}ms", ms).yellow()
    } else {
        format!("{}ms", ms).red()
    };

    println!("{} {} {} {} -> {} ({})", ts.dimmed(), prefix, method.bold(), path, status_styled, dur_styled);
}
