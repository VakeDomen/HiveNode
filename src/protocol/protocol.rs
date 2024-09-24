use std::net::TcpStream;
use anyhow::Result;
use log::info;

use crate::{messages::proxy_request::ProxyRequest, protocol::network_util::{authentiate, poll, read_next_message, read_next_message_length, stream_response_to_java_proxy}};

pub fn run_protocol() -> Result<()> {
    // Connect to the Java Proxy Server
    let mut stream = TcpStream::connect("prog3.student.famnit.upr.si:7777")?;
    info!("Connected to Java Proxy Server");

    let _  = authentiate(&mut stream)?;
    
    loop {
        // poll for work
        let _ = poll(&mut stream)?;
        let message_length = read_next_message_length(&mut stream)?;
        let raw_message = read_next_message(&mut stream, message_length)?;
        let request = ProxyRequest::from(raw_message);

        if !request.protocol.eq("HIVE") && !request.method.eq("PONG") {
            info!("Recieved request: {:#?}", request);
        }

        if !request.protocol.eq("HIVE") {
            let _ = stream_response_to_java_proxy(request, &mut stream)?;
        }
    }
}