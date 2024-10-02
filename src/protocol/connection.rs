use std::{env, net::TcpStream};
use anyhow::Result;
use log::info;

use crate::{messages::proxy_request::ProxyRequest, models::{poller::Poller, tags::Tags}, protocol::network_util::{authenticate, make_ollama_request, poll, proxy}};

pub fn run_protocol() -> Result<()> {
    
    // Connect to the Proxy Server
    let proxy_server_url = env::var("HIVE_CORE_URL").expect("HIVE_CORE_URL");
    let mut stream = TcpStream::connect(proxy_server_url.clone())?;
    info!("Establised connection to HiveCore Proxy Server: {}", proxy_server_url);


    let req = ProxyRequest::new_http_get("/api/tags");
    let resp = make_ollama_request(&req)?;
    let tags = Tags::try_from(resp)?;
    let mut poller = Poller::from(tags);


    authenticate(&mut stream)?;
    info!("Succesfully authenticated to the proxy");
    loop {
        poll(&mut stream, poller.next().unwrap())?;
        proxy(&mut stream)?;
    }
}

