use std::{env, net::TcpStream};
use anyhow::Result;
use log::info;

use crate::protocol::network_util::{authenticate, create_poller, poll, proxy};

pub fn run_protocol() -> Result<()> {
    
    // Connect to the Proxy Server
    let proxy_server_url = env::var("HIVE_CORE_URL").expect("HIVE_CORE_URL");
    let mut stream = TcpStream::connect(proxy_server_url.clone())?;
    info!("Establised connection to HiveCore Proxy Server: {}", proxy_server_url);

    let mut poller = create_poller()?;

    authenticate(&mut stream)?;
    info!("Succesfully authenticated to the proxy");
    loop {
        poll(&mut stream, poller.next().unwrap())?;
        let should_refresh = proxy(&mut stream)?;
        if should_refresh {
            poller = create_poller()?;
        }
    }
}

