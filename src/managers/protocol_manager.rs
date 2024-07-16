
use log::error;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::ws::messages::{message::{IncommingMessage, OutgoingMessage}, message_type::IncommingMessageType};

#[derive(Debug, PartialEq)]
pub enum State {
    Offline,
    Unauthenticated,
    Authenticated,
    Ready,
}


pub struct ProtocolManager {
    sender: Sender<OutgoingMessage>,
    reciever: Receiver<IncommingMessage>,
    state: State,
}

impl ProtocolManager {
    pub fn new(sender: Sender<OutgoingMessage>, reciever: Receiver<IncommingMessage>) -> Self {
        Self {
            sender,
            reciever,
            state: State::Offline,
        }
    }
}


impl ProtocolManager {
    pub async fn start(mut self) {
        if self.state != State::Offline {
            error!("Protocol manager already running.");
            return;
        }

        loop {
            match self.reciever.recv().await {
                Some(message) => self.handle_incomming_message(message),
                None => {
                    error!("Unknown message when recieving a message with protocol manager");
                    continue;
                },
            }
        }
    }    
    
    
    fn handle_incomming_message(&mut self, message: IncommingMessage) {
        match self.state {
            State::Offline => match message.message_type {
                IncommingMessageType::LoadModels => todo!(),
                IncommingMessageType::SubmitEmbed => todo!(),
                IncommingMessageType::SubmitPrompt => todo!(),
                IncommingMessageType::Error => todo!(),
            },
            State::Unauthenticated => match message.message_type {
                IncommingMessageType::LoadModels => todo!(),
                IncommingMessageType::SubmitEmbed => todo!(),
                IncommingMessageType::SubmitPrompt => todo!(),
                IncommingMessageType::Error => todo!(),
            },
            State::Authenticated => match message.message_type {
                IncommingMessageType::LoadModels => todo!(),
                IncommingMessageType::SubmitEmbed => todo!(),
                IncommingMessageType::SubmitPrompt => todo!(),
                IncommingMessageType::Error => todo!(),
            },
            State::Ready => match message.message_type {
                IncommingMessageType::LoadModels => todo!(),
                IncommingMessageType::SubmitEmbed => todo!(),
                IncommingMessageType::SubmitPrompt => todo!(),
                IncommingMessageType::Error => todo!(),
            },
        }
    }
}


