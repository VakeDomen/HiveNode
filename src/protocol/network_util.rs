use std::net::TcpStream;
use anyhow::Result;
use log::error;
use reqwest::blocking::Client;
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::blocking::Response;
use std::io::{BufRead, BufReader, Read, Write};

use crate::messages::proxy_request::ProxyRequest;


pub fn authentiate(stream: &mut TcpStream) -> Result<()> {
    // Create an HTTP client
    stream.write_all(b"AUTH node-worker-1 HIVE\r\n")?;
    stream.flush()?;
    Ok(())
}

pub fn poll(stream: &mut TcpStream) -> Result<()> {
    // Create an HTTP client
    stream.write_all(b"POLL mistral-nemo HIVE\r\n")?;
    stream.flush()?;
    Ok(())
}

pub fn read_next_message_length(stream: &mut TcpStream) -> Result<usize> {
    let mut len_buf = [0u8; 4];
    if let Err(e) = stream.read_exact(&mut len_buf) {
        error!("Error reading length: {}", e);
        return Err(e.into());
    }
    Ok(i32::from_be_bytes(len_buf) as usize)
}

pub fn read_next_message(stream: &mut TcpStream, message_length: usize) -> Result<String> {
    let mut buffer = vec![0u8; message_length];
    if let Err(e) = stream.read_exact(&mut buffer) {
        error!("Error reading message: {}", e);
        return Err(e.into());
    }

    let raw_request = String::from_utf8_lossy(&buffer);
    let raw_request = raw_request.into_owned();
    Ok(raw_request)
}



pub fn stream_response_to_java_proxy(
    request: ProxyRequest,
    stream: &mut TcpStream,
) -> Result<()> {
    let ollama_url = format!("http://localhost:11434{}", request.uri);
    let response = make_ollama_request(ollama_url, &request)?;
    let _ = write_http_status_line(stream, &response)?;
    let _ = write_http_headers(stream, &response)?;
    let _ = stream_body(stream, response)?;
    Ok(())
}

fn stream_body(stream: &mut TcpStream, response: Response) -> Result<()> {
    let mut response_reader = BufReader::new(response);

    loop {
        let mut chunk = Vec::new();
        let bytes_read = response_reader.read_until(b'\n', &mut chunk)?;
        if bytes_read == 0 {
            break;
        }

        let chunk_size = format!("{:X}\r\n", bytes_read);
        stream.write_all(chunk_size.as_bytes())?;
        stream.write_all(&chunk)?;
        stream.write_all(b"\r\n")?;
        stream.flush()?;
    }
    stream.write_all(b"0\r\n\r\n")?;
    stream.flush()?;
    Ok(())
}

fn write_http_headers(stream: &mut TcpStream, response: &Response) -> Result<()> {
    for (key, value) in response.headers() {
        if key.as_str().to_ascii_lowercase() != "transfer-encoding" {
            let header_line = format!("{}: {}\r\n", key, value.to_str()?);
            stream.write_all(header_line.as_bytes())?;
        }
    }
    stream.write_all(b"Transfer-Encoding: chunked\r\n")?;
    stream.write_all(b"Connection: close\r\n")?;
    stream.write_all(b"\r\n")?; // End of headers
    stream.flush()?;
    Ok(())
}

fn write_http_status_line(stream: &mut TcpStream, response: &Response) -> Result<()> {
    // Write the status line
    let status_line = format!(
        "HTTP/1.1 {} {}\r\n", 
        response.status(), 
        response
            .status()
            .canonical_reason()
            .unwrap_or("")
    );
    stream.write_all(status_line.as_bytes())?;
    stream.flush()?;
    Ok(())
}
 
fn make_ollama_request(server_location: String, request: &ProxyRequest) -> Result<Response> {
    let client = Client::new();
    let mut request_builder = client.request(request.method.parse().unwrap(), &server_location);

    // Exclude certain headers when forwarding
    for (key, value) in request.headers.iter() {
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
    if !request.body.is_empty() {
        request_builder = request_builder.body(request.body.to_string());
    }

    // Send the request and get the response
    Ok(request_builder.send()?)
}