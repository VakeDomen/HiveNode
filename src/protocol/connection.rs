use std::{env, net::TcpStream, sync::{Arc, Mutex, RwLock}};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Days, Utc};
use once_cell::sync::Lazy;
use reqwest::blocking::Client;
use lazy_static::lazy_static;

use crate::protocol::network_util::{authenticate, poll, proxy};

use super::network_util::get_tags;



lazy_static! {
    static ref LAST_REFRESH: Arc<RwLock<DateTime<Utc>>> = Arc::new(RwLock::new(Utc::now()));
}


pub fn run_protocol(nonce: u64) -> Result<()> {
    let proxy_server_url = env::var("HIVE_CORE_URL").expect("HIVE_CORE_URL");
    let mut stream = TcpStream::connect(proxy_server_url.clone())?;
    let client = Client::new();
    let mut local_refresh_time: DateTime<Utc> = init_local_time();

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

        if let Err(e) = poll(&mut stream, &models, &false) {
            return Err(anyhow!(format!("Error polling HiveCore: {}", e)));
        };

        let should_refresh = match proxy(&mut stream, &client) {
            Ok(should_refresh) => should_refresh,
            Err(e) => return Err(anyhow!(format!("Failed to proxy request: {}", e))),
        };

        if should_refresh {
            notify_refresh()
        }
    }
}

fn notify_refresh() {
    let mut last_refresh = LAST_REFRESH.write().unwrap();
    *last_refresh = Utc::now();
}

fn init_local_time() -> DateTime<Utc> {
    Utc::now().checked_sub_days(Days::new(1)).unwrap()
}

fn get_last_refresh() -> DateTime<Utc> {
    *LAST_REFRESH.read().unwrap()
}

fn refresh_poll_models(
    client: &Client, 
    local_last_refresh: &mut DateTime<Utc>,
    models: &mut String,
) -> Result<()> {
    *local_last_refresh = get_last_refresh();
    Ok(())
}

