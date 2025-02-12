use std::{env, net::TcpStream};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use reqwest::blocking::Client;

use crate::protocol::network_util::{authenticate, poll, proxy};
use super::state::{get_last_refresh, init_local_time, notify_refresh, refresh_poll_models};



pub fn run_protocol(nonce: u64) -> Result<()> {
    let proxy_server_url = env::var("HIVE_CORE_URL").expect("HIVE_CORE_URL");
    let mut stream = TcpStream::connect(proxy_server_url.clone())?;
    let client = Client::new();
    let mut local_refresh_time: DateTime<Utc> = init_local_time();
    let mut opzimized_poll = false;
    let mut models ="/".to_string();

    if let Err(e) = refresh_poll_models(&client, &mut local_refresh_time, &mut models) {
        return Err(anyhow!(format!("Error refreshing available models: {}", e)));
    };

    if let Err(e) = authenticate(&mut stream, nonce, &client) {
        return Err(anyhow!(format!("Error authenticating: {}", e)));
    };
    
    loop {
        let global_refresh_time = get_last_refresh();

        if global_refresh_time > local_refresh_time {
            if let Err(e) = refresh_poll_models(&client, &mut local_refresh_time, &mut models) {
                return Err(anyhow!(format!("Error refreshing models: {}", e)));
            };
        }

        if let Err(e) = poll(&mut stream, &models, &opzimized_poll) {
            return Err(anyhow!(format!("Error polling HiveCore: {}", e)));
        };

        opzimized_poll = true;

        let should_refresh = match proxy(&mut stream, &client) {
            Ok(should_refresh) => should_refresh,
            Err(e) => return Err(anyhow!(format!("Failed to proxy request: {}", e))),
        };

        if should_refresh {
            notify_refresh()
        }
    }
}
