use chrono::Local;
use colog::{
    basic_builder,
    format::{default_level_color, CologStyle},
};
use log::{Level, LevelFilter, SetLoggerError};

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
    // build the logger
    let mut builder = basic_builder();
    // use your custom prefix formatter
    builder.format(colog::formatter(CustomPrefixToken));
    // default for everything (None) is Debug
    builder.filter(None, LevelFilter::Debug);
    // but “bollard” and “hyper” only at Warn+
    builder.filter(Some("bollard"), LevelFilter::Warn);
    builder.filter(Some("hyper"), LevelFilter::Warn);
    // (optional) dial down reqwest if it’s chatty
    builder.filter(Some("reqwest"), LevelFilter::Warn);
    // now install it
    builder.init();
    // and set the global max to Info (so your own logs at Debug will still get through
    // only if you explicitly log at Debug; otherwise Info+)
    log::set_max_level(LevelFilter::Info);
    Ok(())
}
