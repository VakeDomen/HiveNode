use std::{env, net::TcpStream};
use anyhow::Result;
use log::info;

use crate::protocol::network_util::{authenticate, poll, proxy};

pub fn run_protocol() -> Result<()> {
    
    // Connect to the Proxy Server
    let proxy_server_url = env::var("HIVE_CORE_URL").expect("HIVE_CORE_URL");
    let mut stream = TcpStream::connect(proxy_server_url.clone())?;
    info!("Establised connection to HiveCore Proxy Server: {}", proxy_server_url);

    authenticate(&mut stream)?;
    loop {
        poll(&mut stream)?;
        proxy(&mut stream)?;
    }
}

