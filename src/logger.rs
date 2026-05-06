use log::{Log, Metadata, Record};

pub struct Logger;

pub static LOGGER: Logger = Logger;

pub fn log_level_from_env() -> log::LevelFilter {
    if let Ok(env) = std::env::var("RUSTGC_LOG") {
        match env.to_lowercase().as_str() {
            "trace" => log::LevelFilter::Trace,
            "debug" => log::LevelFilter::Debug,
            "info" => log::LevelFilter::Info,
            "warn" => log::LevelFilter::Warn,
            "error" => log::LevelFilter::Error,
            _ => log::LevelFilter::Off,
        }
    } else {
        log::LevelFilter::Off
    }
}

impl Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{}", record.args());
        }
    }

    fn flush(&self) {
    }
}
