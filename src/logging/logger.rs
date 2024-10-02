use chrono::Local;
use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};
use colog::format::{default_level_color, CologStyle};

struct SimpleLogger;
impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

pub struct CustomPrefixToken;
impl CologStyle for CustomPrefixToken {
    fn prefix_token(&self, level: &Level) -> String {
        format!(
            "[{}] [{}]",
            // self.level_color(level, self.),
            default_level_color(level, level.as_str()),
            Local::now()
        )
    }
}

pub fn init_logging() -> Result<(), SetLoggerError> {
    // colog::init();
    let mut builder = colog::basic_builder();
    builder.format(colog::formatter(CustomPrefixToken));
    builder.filter(None, LevelFilter::Debug);
    builder.init();
    log::set_logger(&SimpleLogger).map(|()| log::set_max_level(LevelFilter::Info))
}