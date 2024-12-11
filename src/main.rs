use dotenv::dotenv;
use futures::prelude::stream;
use influxdb2::{models::DataPoint, Client};
use log::{error, warn};
use logging::logger::init_logging;
use machine_info::Machine;
use nvml_wrapper::error::NvmlError;
use nvml_wrapper::Nvml;
use protocol::connection::run_protocol;
use std::env::VarError;
use std::sync::Arc;
use std::thread::{sleep, spawn};
use std::time::Duration;
use std::{env, thread};
use sysinfo::System;
use tokio::runtime::Handle;

mod logging;
mod messages;
mod models;
mod protocol;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = init_logging();
    let _ = dotenv();
    start_influx_logging(Handle::current());

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

struct Error {
    message: String,
}

impl From<VarError> for Error {
    fn from(value: VarError) -> Self {
        Self {
            message: format!("{:?}", value),
        }
    }
}
impl From<NvmlError> for Error {
    fn from(value: NvmlError) -> Self {
        Self {
            message: format!("{:?}", value),
        }
    }
}

fn start_influx_logging(tokio_handle: Handle) {
    let _ = thread::Builder::new()
        .name("influx_logging".to_string())
        .spawn(move || -> Result<(), Error> {
            let nvml = Nvml::init()?;
            // Get the first `Device` (GPU) in the system
            let device = nvml.device_by_index(0)?;

            println!("{:?}", device);
            let mut machine = Machine::new();
            let mut system = System::new_all();
            let host = env::var("INFLUX_HOST")?;
            let org = env::var("INFLUX_ORG")?;
            let token = env::var("INFLUX_TOKEN")?;
            let client = Arc::new(Client::new(host, org, token));
            let node_key = env::var("HIVE_KEY").expect("Missing HIVE_KEY variable.");
            loop {
                let mut data_points = vec![];
                for gpu_usage in machine.graphics_status() {
                    data_points.extend([DataPoint::builder("gpu")
                        .tag("id", gpu_usage.id)
                        .field("memory_usage", gpu_usage.memory_used as f64)
                        .field("temperature", gpu_usage.temperature as f64)]);
                }

                if let Ok(status) = machine.system_status() {
                    println!("Status is ok! {:?}", status);
                    data_points.extend([
                        DataPoint::builder("cpu").field("usage", status.cpu as i64),
                        DataPoint::builder("memory").field("used", status.memory as i64),
                    ]);
                }
                let data: Vec<DataPoint> = data_points
                    .into_iter()
                    .filter_map(|x| x.tag("node", &node_key).build().ok())
                    .collect();
                let clone = client.clone();
                tokio_handle.spawn(async move {
                    let _ = clone.write("hivecore", stream::iter(data)).await;
                });
                sleep(Duration::from_secs(5));
                system.refresh_all();
            }
        });
}
