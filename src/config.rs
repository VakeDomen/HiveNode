use std::env;
use once_cell::sync::Lazy;

pub static HIVE_CORE_URL: Lazy<String> = Lazy::new(|| env::var("HIVE_CORE_URL").expect("Enviroment variable HIVE_CORE_URL not set."));
pub static VERBOSE_SOCKETS: Lazy<bool> = Lazy::new(|| env::var("VERBOSE_SOCKETS").unwrap_or("true".to_string()).eq("true"));
