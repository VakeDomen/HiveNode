use dotenv::dotenv;
use log::{error, warn};
use logging::logger::init_logging;
use logging::setup_influx_logging;
use protocol::connection::run_protocol;
use protocol::state::{get_shutdown, set_reboot};
use std::env;
use std::thread::{sleep, spawn};
use std::time::Duration;
use tokio::runtime::Handle;

mod logging;
mod messages;
mod models;
mod protocol;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = init_logging();
    let _ = dotenv();
    if let Err(e) = setup_influx_logging(Handle::current()) {
        println!("Not setting up influx: {}", e.message);
    }

    let concurrent = env::var("CONCURRENT_RQEUESTS")
        .expect("CONCURRENT_RQEUESTS")
        .parse()
        .unwrap();

    let nonce = rand::random::<u64>();
    let reconnection_duration_seconds = 10;

    let mut handles = vec![];
    for _ in 0..concurrent {
        let movable_nonce = nonce.clone();
        let handle = spawn(move || loop {
            if let Err(e) = run_protocol(movable_nonce) {
                error!("Connection to proxy ended: {}", e);
                warn!(
                    "Waiting {}s before reconnection.",
                    reconnection_duration_seconds
                );
            }
            if get_shutdown() {
                break;
            }
            set_reboot(false);
            sleep(Duration::from_secs(reconnection_duration_seconds));
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.join();
    }

    Ok(())
}
