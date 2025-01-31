use colored::Colorize;
use log::{Level, Log};

pub static LOGGER: Logger = Logger;

pub struct Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &log::Record) {
        eprintln!("{}{}", format_level(&record.level()), record.args());
    }

    fn flush(&self) {}
}

fn format_level(level: &log::Level) -> String {
    match level {
        log::Level::Error => "❌ Error: ".bold().red().to_string(),
        log::Level::Warn => "⚠️  Warning: ".bold().yellow().to_string(),
        log::Level::Info => String::new(),
        log::Level::Debug => "Debug: ".bold().blue().to_string(),
        log::Level::Trace => "Trace: ".bold().purple().to_string(),
    }
}
