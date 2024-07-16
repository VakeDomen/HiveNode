use log::error;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::ws::messages::message::{IncommingMessage, OutgoingMessage};


pub struct ProtocolManager {
    sender: Sender<OutgoingMessage>,
    reciever: Receiver<IncommingMessage>,
}

impl ProtocolManager {
    pub fn new(sender: Sender<OutgoingMessage>, reciever: Receiver<IncommingMessage>) -> Self {
        Self {
            sender,
            reciever,
        }
    }

    pub async fn start(mut self) {
        loop {
            match self.reciever.recv().await {
                Some(message) => handle_incomming_message(message),
                None => {
                    error!("Unknown message when recieving a message with protocol manager");
                    continue;
                },
            }
        }
    }
}

fn handle_incomming_message(message: IncommingMessage) {
    todo!()
}