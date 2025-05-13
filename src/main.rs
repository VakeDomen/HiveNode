use dotenv::dotenv;
use log::{error, warn};
use logging::logger::init_logging;
use logging::setup_influx_logging;
use protocol::connection::run_protocol;
use protocol::docker::start_ollama_docker;
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
    std::env::set_var("RUST_LOG", "hive_node=debug,bollard=warn,hyper=warn");
    let _ = init_logging();
    let _ = dotenv();
    let _ = setup_influx_logging(Handle::current());

    // 1. bring up Ollama in Docker
    let models_dir = env::var("HIVE_OLLAMA_MODELS")
        .expect("HIVE_OLLAMA_MODELS must be set");
    let container_id = start_ollama_docker(&models_dir).await?;
    // point your code at the new local Ollama
    env::set_var("OLLAMA_URL", "http://127.0.0.1:11434");

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

    // 3. tear down the Ollama container before exit
    start_ollama_docker(&container_id).await?;

    Ok(())
}