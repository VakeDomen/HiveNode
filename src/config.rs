use std::env;
use once_cell::sync::Lazy;

pub static HIVE_CORE_URL: Lazy<String> = Lazy::new(|| env::var("HIVE_CORE_URL").expect("Enviroment variable HIVE_CORE_URL not set."));
pub static VERBOSE_SOCKETS: Lazy<bool> = Lazy::new(|| env::var("VERBOSE_SOCKETS").unwrap_or("true".to_string()).eq("true"));


pub const SEED: u64 = 42;

pub const TEMPERATURE: f64 = 0.4;
pub const SAMPLE_LEN: usize = 1000;
pub const TOP_K: Option<usize> = None;
pub const TOP_P: Option<f64> = None;

pub const SPLIT_PROPMT: bool = false;
pub const REPEAT_PENALTY: f32 = 1.1;
pub const REPEAT_LAST_N: usize = 64;