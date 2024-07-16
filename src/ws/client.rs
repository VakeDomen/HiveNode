use anyhow::Result;
use log::{error, info};
use tokio::{net::TcpStream, sync::mpsc::{self, Sender}};
use tokio_tungstenite::{
    connect_async, 
    tungstenite::{http::Response, Error, Message}, 
    WebSocketStream
};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use crate::{
    config::{HIVE_CORE_URL, VERBOSE_SOCKETS}, 
    managers::protocol_manager::ProtocolManager, 
    ws::messages::message::OutgoingMessage
};
use super::messages::{
    message::IncommingMessage, 
    message_type::{OutgoingMessageBody, OutgoingMessageType}, 
    variants::bidirectional::error::ErrorMessage
};


type OutSocket = SplitSink<WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>, Message>;



//                 |                 APPLICATION 
//  ___            |     __________               ___
// | w |      WS   |    |          |     MPSC    | p |
// | s |  ---------=----=->[parse]-=-----------> | r |  
// |   |           |    |    |     |             | o |
// | s |           |    |    |     |             | t |
// | e |           |    |    |MPSC |             | o |
// | r |           |    |    |     |             |   |
// | v |           |    |    |     |             | M |
// | e |      WS   |    |    v     |     MPSC    | a |
// | r |  <--------=----=[respond]<=------------ | n |
// |___|           |    |__________|             |___|
//                 |       CLIENT


pub async fn connect_to_hive() -> Result<()> {
    let (socket, response) = match connect_async(&*HIVE_CORE_URL).await {
        Ok(conn) => conn,
        Err(e) => return Err(e.into()),
    };

    if *VERBOSE_SOCKETS {
        display_connection(&response);
    }

    let (to_manager_sender, to_manager_reciever) = mpsc::channel::<IncommingMessage>(100);
    let (to_ws_sender, mut to_ws_reciever) = mpsc::channel::<OutgoingMessage>(100);
    let (mut write_ws, mut read_ws) = socket.split();
    let out_channel = to_ws_sender.clone();
    let _ = tokio::spawn(async move {
        let pm = ProtocolManager::new(to_ws_sender, to_manager_reciever);
        pm.start().await;
    });

    loop {
        tokio::select! {
            Some(msg) = read_ws.next() => recieve_message_form_server(msg, &out_channel, &to_manager_sender).await,
            Some(msg) = to_ws_reciever.recv() => send_message_to_server(msg, &mut write_ws).await
        }
    }
}


async fn recieve_message_form_server(
    message_result: Result<Message, Error>, 
    server_channel: &Sender<OutgoingMessage>,
    protocol_manager_channel: &Sender<IncommingMessage>,
) {
    match message_result {
        Ok(socket_message) => {
            if *VERBOSE_SOCKETS {
                display_message(&socket_message);
            }
            // Process the incoming message
            // let incoming_message = process_incoming_message(message);
            let message_content = match socket_message {
                Message::Text(message_content) => message_content,
                _ => return send_error_to_server(0, "Can't parse message type".into(), server_channel).await,
            };

            let parsed_message: IncommingMessage =  match serde_json::from_str(&message_content) {
                Ok(message) => message,
                Err(e) => return send_error_to_server(0, format!("Failed while parsing message: {}", e), server_channel).await,
            };

            if let Err(e) = protocol_manager_channel.send(parsed_message).await {
                send_error_to_server(0, format!("Failed passing message to protocol: {}", e), server_channel).await;
            }
        }
        Err(e) => error!("Error reading message: {}", e),
    }
}

async fn send_error_to_server(code: u32, message: String, channel: &Sender<OutgoingMessage>) {
    if let Err(e) = channel.send(OutgoingMessage {
        message_type: OutgoingMessageType::Error,
        task_id: 0.to_string(),
        body: OutgoingMessageBody::Error(ErrorMessage {
            code,
            message,
        }),
    }).await {
        error!("Failed pushing error message to out channel: {}", e)
    };
}

async fn send_message_to_server(
    message: OutgoingMessage, 
    socket_connection: &mut OutSocket,
) {
    match message.try_into() {
        Ok(message) => {
            if let Err(e) = socket_connection.send(message).await {
                eprintln!("Error sending message: {}", e);
            }
        }
        Err(e) => error!("Failed sending message to the server: {}", e),
    }
}

fn display_connection(response: &Response<Option<Vec<u8>>>) {
    info!("************* Established connection: {} *************", response.status());
    info!("SERVER: {}", *HIVE_CORE_URL);
    info!("HEADERS:");
    for (ref header, value) in response.headers() {
        info!("* {}: {:?}", header, value);
    }
    info!("******************************************************\n");
}

fn display_message(message: &Message) {
    info!("****************** Received message ******************");
    info!("BODY:");
    info!("{message}");
    info!("******************************************************\n");
}