use std::{env, net::TcpStream, sync::{Arc, Mutex}};
use anyhow::{anyhow, Result};
use log::info;
use once_cell::sync::Lazy;
use reqwest::blocking::Client;

use crate::protocol::network_util::{authenticate, create_poller, poll, proxy};

pub static MODELS: Lazy<Arc<Mutex<String>>> = Lazy::new(|| Arc::new(Mutex::new("/".into())));

pub fn run_protocol(nonce: u64) -> Result<()> {
    
    // Connect to the Proxy Server
    let proxy_server_url = env::var("HIVE_CORE_URL").expect("HIVE_CORE_URL");
    let mut stream = TcpStream::connect(proxy_server_url.clone())?;
    info!("Establised connection to HiveCore Proxy Server: {}", proxy_server_url);

    let client = Client::new();

    if let Err(e) = refresh_poll_models(&client) {
        return Err(anyhow!(format!("Error refreshing avalible models: {}", e)));
    };

    if let Err(e) = authenticate(&mut stream, nonce, &client) {
        return Err(anyhow!(format!("Error authenticating: {}", e)));
    };

    info!("Succesfully authenticated to the proxy");
    let mut has_informed_core_of_tags = false;

    loop {
        let models = {
            let models = MODELS.lock().unwrap();
            models.clone()
        };
        
        if let Err(e) = poll(&mut stream, models, &has_informed_core_of_tags) {
            return Err(anyhow!(format!("Error polling HiveCore: {}", e)));
        };

        has_informed_core_of_tags = true;
    
        let should_refresh = match proxy(&mut stream, &client) {
            Ok(should_refresh) => should_refresh,
            Err(e) => return Err(anyhow!(format!("Failed to proxy request: {}", e))),
        };

        if should_refresh {
            has_informed_core_of_tags = false;
            refresh_poll_models(&client)?;
        }
    }
}

fn refresh_poll_models(client: &Client) -> Result<()> {
    let poller = create_poller(client)?;
    let mut models = MODELS.lock().unwrap();
    *models = poller.get_models_target();
    Ok(())
}
