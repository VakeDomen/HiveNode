use std::{env, net::TcpStream, sync::{Arc, Mutex}};
use anyhow::Result;
use log::info;
use once_cell::sync::Lazy;

use crate::protocol::network_util::{authenticate, create_poller, poll, proxy};

pub static MODELS: Lazy<Arc<Mutex<String>>> = Lazy::new(|| Arc::new(Mutex::new("/".into())));

pub fn run_protocol(nonce: u64) -> Result<()> {
    
    // Connect to the Proxy Server
    let proxy_server_url = env::var("HIVE_CORE_URL").expect("HIVE_CORE_URL");
    let mut stream = TcpStream::connect(proxy_server_url.clone())?;
    info!("Establised connection to HiveCore Proxy Server: {}", proxy_server_url);

    refresh_poll_models()?;
    authenticate(&mut stream, nonce)?;

    info!("Succesfully authenticated to the proxy");
    
    loop {
        let models = {
            let models = MODELS.lock().unwrap();
            models.clone()
        };
        
        poll(&mut stream, models)?;
        let should_refresh = proxy(&mut stream)?;

        if should_refresh {
            refresh_poll_models()?;
        }
    }
}

fn refresh_poll_models() -> Result<()> {
    let poller = create_poller()?;
    let mut models = MODELS.lock().unwrap();
    *models = poller.get_models_target();
    Ok(())
}
