use dotenv::dotenv;
use futures::prelude::stream;
use influxdb2::{models::DataPoint, Client};
use log::{error, warn};
use logging::logger::init_logging;
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
            let mut system = System::new_all();
            let host = env::var("INFLUX_HOST")?;
            let org = env::var("INFLUX_ORG")?;
            let token = env::var("INFLUX_TOKEN")?;
            let client = Arc::new(Client::new(host, org, token));
            let node_key = env::var("HIVE_KEY").expect("Missing HIVE_KEY variable.");
            let total_gpus = nvml.device_count()?;

            loop {
                let mut data_points = vec![];

                for i in 0..total_gpus {
                    let device = nvml.device_by_index(i)?;
                    let fan_speed = device.fan_speed(0)?; // Currently 17% on my system
                    let power_limit = device.enforced_power_limit()?; // 275k milliwatts on my system
                    let encoder_util = device.encoder_utilization()?; // Currently 0 on my system; Not encoding anything
                    let memory_info = device.memory_info()?; // Currently 1.63/6.37 GB used on my system

                    data_points.extend([DataPoint::builder("gpu")
                        .tag("index", i.to_string())
                        .field("memory_used", memory_info.used as f64)
                        .field("memory_free", memory_info.free as f64)
                        .field("memory_total", memory_info.total as f64)
                        .field("fan_speed", fan_speed as f64)
                        .field("power_limit", power_limit as f64)
                        .field("encoder_util", encoder_util.utilization as f64)
                        .field("sampling_period", encoder_util.sampling_period as f64)
                        .field(
                            "energy_consumption",
                            device.total_energy_consumption()? as f64,
                        )]);
                }

                for cpu in system.cpus() {
                    data_points
                        .push(DataPoint::builder("cpu").field("usage", cpu.cpu_usage() as f64));
                }

                data_points.push(
                    DataPoint::builder("memory")
                        .field("free", system.free_memory() as i64)
                        .field("used", system.used_memory() as i64)
                        .field("total", system.total_memory() as i64)
                        .field("swap_free", system.free_swap() as i64)
                        .field("swap_used", system.used_swap() as i64)
                        .field("swap_total", system.total_swap() as i64),
                );

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
