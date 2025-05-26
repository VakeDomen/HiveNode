use dotenv::dotenv;
use log::{error, warn};
use logging::logger::init_logging;
use logging::setup_influx_logging;
use protocol::connection::run_protocol;
use protocol::docker::start_ollama_docker;
use protocol::state::{get_shutdown, set_reboot};
use tokio::task;
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
    std::env::set_var("RUST_LOG", "hive_node=debug,bollard=warn,hyper=warn");
    let _ = init_logging();
    let _ = dotenv();
    let _ = setup_influx_logging(Handle::current());

    // 1. bring up Ollama in Docker
    
    let container_id = start_ollama_docker().await?;
    // point your code at the new local Ollama
    let ollama_port = env::var("OLLAMA_PORT").expect("OLLAMA_PORT");
    env::set_var("OLLAMA_URL", format!("http://127.0.0.1:{}", ollama_port));

    // 2. spin up your worker threads as before
    let concurrent = env::var("CONCURRENT_REQUESTS")
        .expect("CONCURRENT_REQUESTS").parse::<usize>()?;
    let nonce = rand::random::<u64>();
    let reconnect_secs = 10;
    let mut handles = Vec::with_capacity(concurrent);
    for _ in 0..concurrent {
        let movable_nonce = nonce;
        handles.push(spawn(move || loop {
            if let Err(e) = run_protocol(movable_nonce) {
                error!("Connection to proxy ended: {}", e);
                warn!("Waiting {}s before reconnecting", reconnect_secs);
            }
            if get_shutdown() {
                break;
            }
            set_reboot(false);
            sleep(Duration::from_secs(reconnect_secs));
        }));
    }

    
    // wait for all threads to finish
    for h in handles {
        let _ = h.join();
    }

    Ok(())
}