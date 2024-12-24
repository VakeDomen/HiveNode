use dotenv::dotenv;
use influxdb2::models::DataPoint;
use log::{error, warn};
use logging::logger::init_logging;
use logging::{log_influx, setup_influx_logging};
use once_cell::sync::Lazy;
use protocol::connection::run_protocol;
use std::env;
use std::fmt::format;
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn};
use std::time::Duration;
use tokio::runtime::Handle;

mod logging;
mod messages;
mod models;
mod protocol;

pub static NONCE: Lazy<u64> = Lazy::new(|| rand::random::<u64>());
pub static USERNAME: Lazy<Arc<Mutex<Option<String>>>> = Lazy::new(|| Default::default());

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = init_logging();
    let _ = dotenv();
    let _ = setup_influx_logging(Handle::current());

    let concurrent = env::var("CONCURRENT_RQEUESTS")
        .expect("CONCURRENT_RQEUESTS")
        .parse()
        .unwrap();

    let reconnection_duration_seconds = 10;

    let mut handles = vec![];
    for _ in 0..concurrent {
        let movable_nonce = NONCE.clone();
        let handle = spawn(move || loop {
            if let Err(e) = run_protocol(movable_nonce) {
                if let Ok(guard) = USERNAME.lock() {
                    if let Some(username) = &*guard {
                        log_influx(
                            vec![DataPoint::builder("ollama").field("error", format!("{:?}", e))],
                            username.clone(),
                        );
                    }
                }
                error!("Connection to proxy ended: {}", e);
                warn!(
                    "Waiting {}s before reconnection.",
                    reconnection_duration_seconds
                );
            }
            sleep(Duration::from_secs(reconnection_duration_seconds));
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.join();
    }

    Ok(())
}
