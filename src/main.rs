use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::thread::sleep;
use std::time::Duration;
use log::{error, info, warn};
use logging::logger::init_logging;
use reqwest::blocking::Client;
use reqwest::header::{HeaderName, HeaderValue};
use std::collections::HashMap;
use std::str;
use anyhow::Result;

mod config;
mod logging;

fn main() -> anyhow::Result<()> {
    let _ = init_logging();
    let mut reconnect_count = 0;

    loop {
        if let Err(e) = work() {
            warn!("Connection to proxy ended: {}", e);
            warn!("Waiting {}s before reconnection.", 10 * reconnect_count);
        }
        sleep(Duration::from_secs(10 * reconnect_count));
        reconnect_count += 1;
    }
}

fn work() -> Result<()> {
    // Connect to the Java Proxy Server
    let mut stream = TcpStream::connect("127.0.0.1:7777")?;
    info!("Connected to Java Proxy Server");

    if let Err(e) = authentiate(&mut stream) {
        error!("Error authenticating to the proxy: {}", e);
        return Err(e);
    }
    
    loop {
        // poll for work
        if let Err(e) = poll(&mut stream) {
            error!("Error polling the proxy: {}", e);
            return Err(e);
        }

        // Read the length of the incoming message (4 bytes)
        let mut len_buf = [0u8; 4];
        if let Err(e) = stream.read_exact(&mut len_buf) {
            error!("Error reading length: {}", e);
            return Err(e.into());
        }
        let message_length = i32::from_be_bytes(len_buf) as usize;

        // Read the message
        let mut buffer = vec![0u8; message_length];
        if let Err(e) = stream.read_exact(&mut buffer) {
            error!("Error reading message: {}", e);
            return Err(e.into());
        }

        // Convert buffer to string
        let request_str = String::from_utf8_lossy(&buffer);

        // Parse the HTTP request
        let (protocol, method, uri, headers, body) = parse_request(&request_str);
        if !protocol.eq("HIVE") {
            // Send the request to Ollama API and stream the response back
            if let Err(e) = stream_response_to_java_proxy(&method, &uri, &headers, &body, &mut stream) {
                error!("Error streaming response: {}", e);
                return Err(e);
            }
        }

    }
}

fn authentiate(stream: &mut TcpStream) -> Result<()> {
    // Create an HTTP client
    stream.write_all(b"AUTH node-worker-1 HIVE\r\n")?;
    stream.flush()?;
    Ok(())
}

fn poll(stream: &mut TcpStream) -> Result<()> {
    // Create an HTTP client
    stream.write_all(b"POLL mistral-nemo HIVE\r\n")?;
    stream.flush()?;
    Ok(())
}


fn parse_request(request: &str) -> (String, String, String, HashMap<String, String>, String) {
    // Same as before
    let mut lines = request.lines();
    let request_line = lines.next().unwrap_or("");
    let mut headers = HashMap::new();
    let mut body = String::new();

    // Parse method and URI
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or("").to_string();
    let uri = request_parts.next().unwrap_or("").to_string();
    let protocol = request_parts.next().unwrap_or("").to_string();

    // Parse headers
    for line in &mut lines {
        if line.is_empty() {
            // End of headers
            break;
        }
        let mut parts = line.splitn(2, ": ");
        let name = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("");
        headers.insert(name.to_string(), value.to_string());
    }

    // Read the body
    body = lines.collect::<Vec<&str>>().join("\n");

    if !protocol.eq("HIVE") && !method.eq("PONG") {
        info!("Recieved request:\n\tPROTOCOL: {protocol}\n\tMETHOD: {method}\n\tURI: {uri}\n\tHEADERS: {:#?}\n\tBODY: {body}", headers);
    }
    

    (protocol, method, uri, headers, body)
}


fn stream_response_to_java_proxy(
    method: &str,
    uri: &str,
    headers: &HashMap<String, String>,
    body: &str,
    stream: &mut TcpStream,
) -> Result<()> {
    let ollama_url = format!("http://localhost:11434{}", uri);
    let client = Client::new();
    let mut request_builder = client.request(method.parse().unwrap(), &ollama_url);

    // Exclude certain headers when forwarding
    for (key, value) in headers.iter() {
        let key_lower = key.to_ascii_lowercase();
        if key_lower != "host" && key_lower != "content-length" {
            if let (Ok(header_name), Ok(header_value)) = (
                HeaderName::from_bytes(key.as_bytes()),
                HeaderValue::from_str(value),
            ) {
                request_builder = request_builder.header(header_name, header_value);
            }
        }
    }

    // Set body
    if !body.is_empty() {
        request_builder = request_builder.body(body.to_string());
    }

    // Send the request and get the response
    let response = request_builder.send()?;

    // Write the status line
    let status_line = format!("HTTP/1.1 {} {}\r\n", response.status(), response.status().canonical_reason().unwrap_or(""));
    stream.write_all(status_line.as_bytes())?;

    // Write headers, excluding 'Transfer-Encoding'
    for (key, value) in response.headers() {
        if key.as_str().to_ascii_lowercase() != "transfer-encoding" {
            let header_line = format!("{}: {}\r\n", key, value.to_str()?);
            stream.write_all(header_line.as_bytes())?;
        }
    }

    // Add 'Transfer-Encoding: chunked' and 'Connection: close' headers
    stream.write_all(b"Transfer-Encoding: chunked\r\n")?;
    stream.write_all(b"Connection: close\r\n")?;
    stream.write_all(b"\r\n")?; // End of headers

    stream.flush()?;

    // Read the response body from the upstream server and re-chunk it properly
    let mut response_reader = BufReader::new(response);

    loop {
        let mut chunk = Vec::new();
        let bytes_read = response_reader.read_until(b'\n', &mut chunk)?;
        if bytes_read == 0 {
            break;
        }

        // Write chunk size in hexadecimal followed by \r\n
        let chunk_size = format!("{:X}\r\n", bytes_read);
        stream.write_all(chunk_size.as_bytes())?;
        // Write chunk data
        stream.write_all(&chunk)?;
        // Write \r\n
        stream.write_all(b"\r\n")?;
        stream.flush()?;
    }

    // Write the last chunk (size zero) to signal end of chunks
    stream.write_all(b"0\r\n\r\n")?;
    stream.flush()?;

    Ok(())
}
