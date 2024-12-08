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
    let reconnection_duration_seconds = 10;

    let mut handles = vec![];
    for _ in 0..concurrent {
        
        let movable_nonce = nonce.clone();
        let handle = spawn(move || {
            loop {
                if let Err(e) = run_protocol(movable_nonce) {
                    error!("Connection to proxy ended: {}", e);
                    warn!("Waiting {}s before reconnection.", reconnection_duration_seconds);
                }
                sleep(Duration::from_secs(reconnection_duration_seconds));
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.join();
    }

    Ok(())
}