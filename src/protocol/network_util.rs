use anyhow::{anyhow, Result};
use influxdb2::models::DataPoint;
use log::{error, info, warn};
use reqwest::blocking::Client;
use reqwest::blocking::Response;
use reqwest::header::USER_AGENT;
use reqwest::header::{HeaderName, HeaderValue};
use std::env;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

use crate::logging::log_influx;
use crate::messages::proxy_message::ProxyMessage;
use crate::models::poller::Poller;
use crate::models::tags::Tags;
use crate::models::tags::Version;
use crate::NONCE;
use crate::USERNAME;

pub fn authenticate(stream: &mut TcpStream, nonce: u64, client: &Client) -> Result<String> {
    let key = env::var("HIVE_KEY").expect("HIVE_KEY");
    let ollama_version = get_ollama_version(client);
    let node_version: &str = env!("CARGO_PKG_VERSION");

    // Create an HTTP client
    let auth_request = format!("AUTH {key};{nonce};{node_version};{ollama_version} HIVE\r\n");
    stream.write_all(auth_request.as_bytes())?;
    stream.flush()?;

    let response = read_next_message(stream)?;
    info!("Authenticated as: {}", response.uri);
    Ok(response.uri)
}

pub fn create_poller(client: &Client) -> Result<Poller> {
    let tags = get_tags(client)?;
    Ok(Poller::from(tags))
}

pub fn poll(
    stream: &mut TcpStream,
    model_name: String,
    optimized_polling_sequence: &bool,
) -> Result<()> {
    // polling with "-" will tell the HiveCore to take the last seen
    // set of models as the possible tags. The Core will optimize the
    // sequence in which the models are polled based on the previously
    // handled work.
    // if the last work was using model X, it will prioratize the model
    // X work requests to handle. This minimizes the amount of switching
    // of models in the worker VRAM.
    // requres the worker to have previously sent the tags to the core,
    // so that the core has the list to work with
    // However, polling with X;Y;Z will set the sequence of models in
    // which the work is polled
    let poll_target = if *optimized_polling_sequence {
        "-".to_string()
    } else {
        model_name
    };

    let poll_string = format!("POLL {poll_target} HIVE\r\n");
    stream.write_all(poll_string.as_bytes())?;
    stream.flush()?;
    Ok(())
}

pub fn proxy(mut stream: &mut TcpStream, client: &Client) -> Result<bool> {
    let request = read_next_message(&mut stream)?;
    match request.protocol.as_str() {
        "HIVE" => handle_hive_request(request, stream),
        _ => stream_response_to_proxy(request, &mut stream, client),
    }
}

fn get_tags(client: &Client) -> Result<Tags> {
    let req = ProxyMessage::new_http_get("/api/tags");
    let resp = make_ollama_request(&req, client)?;
    Ok(Tags::try_from(resp)?)
}

pub fn get_ollama_version(client: &Client) -> String {
    let req = ProxyMessage::new_http_get("/api/tags");
    let resp = match make_ollama_request(&req, client) {
        Ok(resp) => resp,
        Err(_) => return "Unknown".to_string(),
    };
    match Version::try_from(resp) {
        Ok(v) => v.version,
        Err(_) => "Unknown".to_string(),
    }
}

fn handle_hive_request(request: ProxyMessage, _stream: &mut TcpStream) -> Result<bool> {
    if !request.method.eq("PONG") {
        info!("Recieved request from HiveCore: {:#?}", request);
    }
    Ok(false)
}

fn read_next_message_length(stream: &mut TcpStream) -> Result<usize> {
    let mut len_buf = [0u8; 4];
    if let Err(e) = stream.read_exact(&mut len_buf) {
        error!("Error reading next message length from HiveCore: {}", e);
        return Err(e.into());
    }
    Ok(i32::from_be_bytes(len_buf) as usize)
}

fn read_next_message(stream: &mut TcpStream) -> Result<ProxyMessage> {
    let message_length = read_next_message_length(stream)?;
    let mut buffer = vec![0u8; message_length];
    if let Err(e) = stream.read_exact(&mut buffer) {
        error!("Error reading message from HiveCore: {}", e);
        return Err(e.into());
    }

    let raw_request = String::from_utf8_lossy(&buffer);
    let raw_request = raw_request.into_owned();
    Ok(ProxyMessage::from(raw_request))
}

fn stream_response_to_proxy(
    request: ProxyMessage,
    stream: &mut TcpStream,
    client: &Client,
) -> Result<bool> {
    info!("Recieved Ollama request.");
    let id = authenticate(stream, NONCE.clone(), client)?;
    if let Ok(mut guard) = USERNAME.lock() {
        *guard = Some(id.clone());
    }
    let response = make_ollama_request(&request, client)?;
    let response_code = response.status().as_u16();
    let mut influx_stream: Vec<u8> = request.body.clone().into_bytes();

    match response_code {
        200 => info!(
            "Ollama responded with: {} | Streaming back response...",
            response.status()
        ),
        _ => warn!(
            "Ollama responded with: {} | Streaming back response...",
            response.status()
        ),
    };

    if let Err(e) = write_http_status_line(stream, &response, &mut influx_stream) {
        send_influx(influx_stream, id);
        return Err(anyhow!("Error streaming status line to HiveCore: {}", e));
    }

    if let Err(e) = write_http_headers(stream, &response, &mut influx_stream) {
        send_influx(influx_stream, id);
        return Err(anyhow!("Error streaming headers to HiveCore: {}", e));
    }

    if let Err(e) = stream_body(stream, response, &mut influx_stream) {
        send_influx(influx_stream, id);
        return Err(anyhow!("Error streaming body to HiveCore: {}", e));
    }

    // if response_code != 200 {
    send_influx(influx_stream, id);
    // }

    info!("Stream ended. Response done.");

    Ok(request.modifies_poll())
}

fn stream_body(
    stream: &mut TcpStream,
    response: Response,
    influx_stream: &mut Vec<u8>,
) -> Result<()> {
    let mut response_reader = BufReader::new(response);

    loop {
        let mut chunk = Vec::new();
        let bytes_read = response_reader.read_until(b'\n', &mut chunk)?;
        if bytes_read == 0 {
            break;
        }

        let chunk_size = format!("{:X}\r\n", bytes_read).into_bytes();
        write_to_both_streams(stream, influx_stream, &chunk_size)?;
        write_to_both_streams(stream, influx_stream, &chunk)?;
        write_to_both_streams(stream, influx_stream, b"\r\n")?;
        stream.flush()?;
    }
    write_to_both_streams(stream, influx_stream, b"0\r\n\r\n")?;
    stream.flush()?;
    Ok(())
}

/// Writes HTTP headers to both TCP stream and the influx stream, which is used for error reporting.
fn write_http_headers(
    stream: &mut TcpStream,
    response: &Response,
    influx_stream: &mut Vec<u8>,
) -> Result<()> {
    for (key, value) in response.headers() {
        if key.as_str().to_ascii_lowercase() != "transfer-encoding" {
            let header_line = format!("{}: {}\r\n", key, value.to_str()?).into_bytes();
            write_to_both_streams(stream, influx_stream, &header_line)?;
        }
    }
    write_to_both_streams(stream, influx_stream, b"Transfer-Encoding: chunked\r\n")?;
    write_to_both_streams(stream, influx_stream, b"Connection: close\r\n")?;
    write_to_both_streams(stream, influx_stream, b"\r\n")?;
    stream.flush()?;
    Ok(())
}

/// Write HTTP status line to both TCP stream and the influx stream, which is used for error reporting if the status is not 200.
fn write_http_status_line(
    stream: &mut TcpStream,
    response: &Response,
    influx_stream: &mut Vec<u8>,
) -> Result<()> {
    // Write the status line
    let status_line = format!(
        "HTTP/1.1 {} {}\r\n",
        response.status(),
        response.status().canonical_reason().unwrap_or("")
    )
    .into_bytes();
    write_to_both_streams(stream, influx_stream, &status_line)?;
    stream.flush()?;
    Ok(())
}

fn make_ollama_request(request: &ProxyMessage, client: &Client) -> Result<Response> {
    let ollama_base_url = env::var("OLLAMA_URL").expect("OLLAMA_URL");
    let request_target = format!("{ollama_base_url}{}", request.uri);

    if request.protocol.eq("HIVE") {
        return Err(anyhow!("Can't make HIVE requests to Ollama."));
    }

    let mut request_builder = client.request(request.method.parse().unwrap(), request_target);

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

/// Writes to both streams simultaneously. Exists to reduce code duplication.
fn write_to_both_streams(tcp: &mut TcpStream, second: &mut Vec<u8>, data: &[u8]) -> Result<()> {
    tcp.write_all(data)?;
    second.extend_from_slice(data);
    Ok(())
}

/// Report a DataPoint with the message of the stream.
fn send_influx(stream: Vec<u8>, id: String) {
    let data_as_string = String::from_utf8(stream);
    let data_point = match data_as_string {
        Ok(data) => DataPoint::builder("ollama").field("response", data),
        Err(x) => DataPoint::builder("ollama").field("utf8_error", x.to_string()),
    };
    log_influx(vec![data_point], id);
}
