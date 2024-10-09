use std::env;
use std::thread::{sleep, spawn};
use std::time::Duration;
use dotenv::dotenv;
use log::{error, warn};
use logging::logger::init_logging;
use protocol::connection::run_protocol;

mod logging;
mod messages;
mod protocol;
mod models;

fn main() -> anyhow::Result<()> {
    let _ = init_logging();
    let _ = dotenv();

    let concurrent = env::var("CONCURRENT_RQEUESTS")
        .expect("CONCURRENT_RQEUESTS")
        .parse()
        .unwrap();
    

    let nonce = rand::random::<u64>();

    let mut handles = vec![];
    for _ in 0..concurrent {
        let movable_nonce = nonce.clone();
        let handle = spawn(move || {
            let mut reconnect_count = 0;

            loop {
                if let Err(e) = run_protocol(movable_nonce) {
                    error!("Connection to proxy ended: {}", e);
                    warn!("Waiting {}s before reconnection.", 10 * reconnect_count);
                }
                sleep(Duration::from_secs(10 * reconnect_count));
                reconnect_count += 1;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.join();
    }

    Ok(())
}