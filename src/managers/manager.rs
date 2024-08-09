use log::error;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::ws::messages::message::{IncommingMessage, OutgoingMessage};

#[derive(Debug, PartialEq)]
pub enum State {
    Offline,
    Unauthenticated,
    Authenticated,
    Ready,
}

pub trait Manager {
    
    async fn start(&mut self);

    fn get_state(&self) -> &State;
    fn set_state(&mut self, new_state: State) -> bool;
    fn get_reciever_mut(&mut self) -> &mut Receiver<IncommingMessage>;
    fn get_sender_mut(&mut self) -> &mut Sender<OutgoingMessage>;

    async fn handle_incomming_message(&mut self, message: IncommingMessage);
    async fn handle_outgoing_message(
        &mut self,
        message: OutgoingMessage, 
    ) {
        match message.try_into() {
            Ok(message) => {
                if let Err(e) = self.get_sender_mut().send(message).await {
                    eprintln!("Error sending message: {}", e);
                }
            }
            Err(e) => error!("Failed sending message to the server: {}", e),
        }
    }
}