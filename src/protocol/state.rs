use std::sync::{Arc, RwLock};

use chrono::{DateTime, Days, Utc};
use lazy_static::lazy_static;
use reqwest::blocking::Client;
use anyhow::Result;

use crate::models::poller::Poller;

use super::network_util::get_tags;


lazy_static! {
    static ref LAST_REFRESH: Arc<RwLock<DateTime<Utc>>> = Arc::new(RwLock::new(Utc::now()));
    static ref NODE_NAME: Arc<RwLock<String>> = Arc::new(RwLock::new(String::from("Unknown")));
    static ref REBOOT: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));
    static ref SHUTDOWN: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));
}


pub fn set_reboot(b: bool) {
    let mut rbt = REBOOT.write().unwrap();
    *rbt = b;
}

pub fn set_shutdown(b: bool) {
    let mut sht = SHUTDOWN.write().unwrap();
    *sht = b;
}

pub fn get_reboot() -> bool {
    *REBOOT.read().unwrap()
}

pub fn get_shutdown() -> bool {
    *SHUTDOWN.read().unwrap()
}

pub fn set_node_name(name: String) {
    let mut global_name = NODE_NAME.write().unwrap();
    *global_name = name;
}

pub fn get_node_name() -> String {
    NODE_NAME.read().unwrap().to_string()
}

pub fn notify_refresh() {
    let mut last_refresh = LAST_REFRESH.write().unwrap();
    *last_refresh = Utc::now();
}

pub fn init_local_time() -> DateTime<Utc> {
    Utc::now().checked_sub_days(Days::new(1)).unwrap()
}

pub fn get_last_refresh() -> DateTime<Utc> {
    *LAST_REFRESH.read().unwrap()
}

pub fn refresh_poll_models(
    client: &Client, 
    local_last_refresh: &mut DateTime<Utc>,
    models: &mut String,
) -> Result<()> {
    *models = (*Poller::from(get_tags(client)?)
                .get_models_target())
                .to_string();
    *local_last_refresh = get_last_refresh();
    Ok(())
}

