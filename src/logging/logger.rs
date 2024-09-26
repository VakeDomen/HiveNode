use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};

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

pub fn init_logging() -> Result<(), SetLoggerError> {
    colog::init();
    log::set_logger(&SimpleLogger).map(|()| log::set_max_level(LevelFilter::Info))
}