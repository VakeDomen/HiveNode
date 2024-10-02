use std::thread::sleep;
use std::time::Duration;
use dotenv::dotenv;
use log::warn;
use logging::logger::init_logging;
use protocol::connection::run_protocol;

mod logging;
mod messages;
mod protocol;
mod models;

fn main() -> anyhow::Result<()> {
    let _ = init_logging();
    let _ = dotenv();
    let mut reconnect_count = 0;

    loop {
        if let Err(e) = run_protocol() {
            warn!("Connection to proxy ended: {}", e);
            warn!("Waiting {}s before reconnection.", 10 * reconnect_count);
        }
        sleep(Duration::from_secs(10 * reconnect_count));
        reconnect_count += 1;
    }
}