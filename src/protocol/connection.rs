use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use std::{env, net::TcpStream};

use super::{
    docker::DOCKER_UPGRADE_LOCK,
    network_util::{handle_control_request, read_next_message, stream_response_to_proxy},
    state::{
        get_last_refresh, get_reboot, get_shutdown, init_local_time, notify_refresh,
        refresh_poll_models,
    },
};
use crate::protocol::network_util::{authenticate, poll};

pub fn run_protocol(nonce: u64) -> Result<()> {
    let proxy_server_url =
        normalize_core_tcp_addr(&env::var("HIVE_CORE_URL").expect("HIVE_CORE_URL"))?;
    let mut stream = TcpStream::connect(proxy_server_url.clone())?;
    let client = Client::new();
    let mut local_refresh_time: DateTime<Utc> = init_local_time();
    let mut opzimized_poll = false;
    let mut models = "/".to_string();

    {
        let _read_guard = DOCKER_UPGRADE_LOCK.read().unwrap();

        if let Err(e) = refresh_poll_models(&client, &mut local_refresh_time, &mut models) {
            return Err(anyhow!(format!("Error refreshing available models: {}", e)));
        }

        if let Err(e) = authenticate(&mut stream, nonce, &client) {
            return Err(anyhow!(format!("Error authenticating: {}", e)));
        }
    }

    loop {
        let global_refresh_time = get_last_refresh();

        if global_refresh_time > local_refresh_time {
            {
                let _read_guard = DOCKER_UPGRADE_LOCK.read().unwrap();

                if let Err(e) = refresh_poll_models(&client, &mut local_refresh_time, &mut models) {
                    return Err(anyhow!(format!("Error refreshing models: {}", e)));
                };
            }
            opzimized_poll = false;
        }

        if let Err(e) = poll(&mut stream, &models, &opzimized_poll) {
            return Err(anyhow!(format!("Error polling HiveCore: {}", e)));
        };

        opzimized_poll = true;

        let should_refresh_result: Result<bool> = {
            let request = read_next_message(&mut stream)?;
            if request.protocol == "HIVE" {
                handle_control_request(&request, &mut stream)
            } else {
                let _read_guard = DOCKER_UPGRADE_LOCK.read().unwrap();
                stream_response_to_proxy(request, &mut stream, &client)
            }
        };

        if let Ok(should_refresh) = should_refresh_result {
            if should_refresh {
                notify_refresh()
            }
        }

        if get_reboot() || get_shutdown() {
            return Ok(());
        }
    }
}

fn normalize_core_tcp_addr(raw_url: &str) -> Result<String> {
    let trimmed = raw_url.trim();
    let without_scheme = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"))
        .unwrap_or(trimmed);
    let addr = without_scheme
        .split('/')
        .next()
        .unwrap_or("")
        .trim()
        .to_string();

    if addr.is_empty() {
        return Err(anyhow!("HIVE_CORE_URL must include host:port"));
    }

    Ok(addr)
}

#[cfg(test)]
mod tests {
    use super::normalize_core_tcp_addr;

    #[test]
    fn normalizes_core_tcp_addr() {
        assert_eq!(
            normalize_core_tcp_addr("http://hivecore.example.com:7777").unwrap(),
            "hivecore.example.com:7777"
        );
        assert_eq!(
            normalize_core_tcp_addr("127.0.0.1:7777").unwrap(),
            "127.0.0.1:7777"
        );
    }
}
