use dotenv::dotenv;
use futures::prelude::stream;
use influxdb2::{models::DataPoint, Client};
use log::{error, warn};
use logging::logger::init_logging;
use protocol::connection::run_protocol;
use std::env;
use std::thread::{sleep, spawn};
use std::time::Duration;

mod logging;
mod messages;
mod models;
mod protocol;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let bucket = "hivecore";
    let host = env::var("INFLUX_HOST").expect("Missing INFLUX_HOST variable.");
    let organisation = env::var("INFLUX_ORG").expect("Missing INFLUX_ORGA variable.");
    let token = env::var("INFLUX_TOKEN").expect("Missing INFLUX_TOKEN variable.");
    let client = Client::new(host, organisation, token);
    for _ in 0..10 {
        let points = [DataPoint::builder("cpu")
            .tag("node", "id-1")
            .field("speed", 100)
            .build()?];

        client.write(bucket, stream::iter(points)).await?;
        println!("Sent one!");
        sleep(Duration::from_millis(100));
    }
    return Ok(());
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
        let handle = spawn(move || loop {
            if let Err(e) = run_protocol(movable_nonce) {
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

async fn submit() {}
