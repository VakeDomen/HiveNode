use dotenv::dotenv;
use futures::prelude::stream;
use influxdb2::models::FieldValue;
use influxdb2::{models::DataPoint, Client};
use log::{error, warn};
use logging::logger::init_logging;
use logging::setup_influx_logging;
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
    setup_influx_logging(Handle::current());

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
