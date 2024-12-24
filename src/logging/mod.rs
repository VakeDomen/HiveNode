use std::{
    env,
    sync::{Arc, LazyLock, Mutex},
    thread::{self, sleep},
    time::Duration,
};

use futures::stream;
use influxdb2::{
    models::{data_point::DataPointBuilder, DataPoint},
    Client,
};
use log::info;
use nvml_wrapper::Nvml;
use sysinfo::System;
use tokio::runtime::Handle;
mod error;
pub use error::*;

use crate::USERNAME;

pub mod logger;

static INFLUX_CLIENT: LazyLock<Arc<Mutex<Option<InfluxInformation>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

struct InfluxInformation {
    client: Client,
    node_key: String,
    tokio_handle: Handle,
}

pub(crate) fn setup_influx_logging(tokio_handle: Handle) -> Result<(), Error> {
    let host = env::var("INFLUX_HOST")?;
    let org = env::var("INFLUX_ORG")?;
    let token = env::var("INFLUX_TOKEN")?;
    let client = Client::new(host, org, token);
    let node_key = env::var("HIVE_KEY")?;

    if let Ok(mut guard) = INFLUX_CLIENT.lock() {
        *guard = Some(InfluxInformation {
            client,
            node_key,
            tokio_handle,
        });
        start_load_logging();
    }
    Ok(())
}

pub(crate) fn log_influx(data: Vec<DataPointBuilder>, id: String) {
    info!("Logging influx!");
    if let Ok(guard) = INFLUX_CLIENT.lock() {
        info!("Influx guard is ready!");
        if let Some(influx) = &*guard {
            info!("Influx information is ready!");
            let clone = influx.client.clone();
            let data: Vec<DataPoint> = data
                .into_iter()
                .filter_map(|x| x.tag("node", &id).build().ok())
                .collect();
            influx.tokio_handle.spawn(async move {
                info!("Writing to the client.");
                let _ = clone.write("hivecore", stream::iter(data)).await;
            });
        }
    }
}

fn start_load_logging() {
    let _ = thread::Builder::new()
        .name("influx_logging".to_string())
        .spawn(move || -> Result<(), Error> {
            let nvml = Nvml::init()?;
            let mut system = System::new_all();
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
                        .field("free", system.free_memory() as f64)
                        .field("used", system.used_memory() as f64)
                        .field("total", system.total_memory() as f64)
                        .field("swap_free", system.free_swap() as f64)
                        .field("swap_used", system.used_swap() as f64)
                        .field("swap_total", system.total_swap() as f64),
                );

                if let Ok(guard) = USERNAME.lock() {
                    if let Some(id) = &*guard {
                        log_influx(data_points, id.clone());
                    }
                }

                sleep(Duration::from_secs(5));
                system.refresh_all();
            }
        });
}
