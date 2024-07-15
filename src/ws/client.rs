use anyhow::Result;
use log::info;
use tokio_tungstenite::tungstenite::{connect, http::Response, Message};

use crate::{config::{HIVE_CORE_URL, VERBOSE_SOCKETS}, ws::messages::message::OutgoingMessage};

pub fn connect_to_hive() -> Result<()> {
    let (mut socket, response) = match connect(&*HIVE_CORE_URL) {
        Ok(conn) => conn,
        Err(e) => return Err(e.into()),
    };

    if *VERBOSE_SOCKETS {
        display_connection(&response);
    }

    let _ = socket.send(OutgoingMessage::default().try_into()?);
    // socket.send(Ou).unwrap();
    loop {
        let msg = socket.read().expect("Error reading message");
        if *VERBOSE_SOCKETS {
            display_message(&msg);
        }
    }
}

fn display_connection(response: &Response<Option<Vec<u8>>>) -> () {
    info!("************* Established connection: {} *************", response.status());
    info!("SERVER: {}", *HIVE_CORE_URL);
    info!("HEADERS:");
    for (ref header, value) in response.headers() {
        info!("* {}: {:?}", header, value);
    }
    info!("******************************************************\n");
}

fn display_message(message: &Message) -> () {
    info!("****************** Received message ******************");
    info!("BODY:");
    info!("{message}");
    info!("******************************************************\n");
}