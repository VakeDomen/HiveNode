
use dashmap::DashMap;
use log::{error, info};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{managers::errors::ProtocolError, ws::messages::{message::{IncommingMessage, OutgoingMessage}, message_type::{IncommingMessageBody, OutgoingMessageBody, OutgoingMessageType}, variants::{incomming::{load_models::LoadModels, submit_embed::SubmitEmbed, submit_prompt::SubmitPrompt}, outgoing::response_load_model::ResponseLoadModel}}};

use super::model_manager::ModelManager;

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
    models: DashMap<String, ModelManager>
}

impl ProtocolManager {
    pub fn new(sender: Sender<OutgoingMessage>, reciever: Receiver<IncommingMessage>) -> Self {
        Self {
            sender,
            reciever,
            state: State::Offline,
            models: DashMap::new(),
        }
    }

    pub async fn start(mut self) {
        if self.state != State::Offline {
            error!("Protocol manager already running.");
            return;
        }

        self.change_state(State::Unauthenticated);

        loop {
            match self.reciever.recv().await {
                Some(message) => self.handle_incomming_message(message).await,
                None => self.reject_incomming_message().await,
            }
        }
    }    

    // PROTOCOL
    async fn handle_incomming_message(&mut self, message: IncommingMessage) {
        let task_id = message.task_id.clone();
        match self.state {
            State::Offline => self.reject_incomming_message().await,
            State::Unauthenticated => match message.body {
                IncommingMessageBody::Success(_message) => self.handle_successfull_authentication().await,
                _ => self.reject_incomming_message().await,
            },
            State::Authenticated => match message.body {
                IncommingMessageBody::LoadModels(message) => self.handle_load_models(message, task_id).await,
                _ => self.reject_incomming_message().await,
            },
            State::Ready => match message.body {
                IncommingMessageBody::SubmitEmbed(message) => self.handle_embedding_request(message).await,
                IncommingMessageBody::SubmitPrompt(message) => self.handle_prompt_request(message).await,
                _ => self.reject_incomming_message().await,
            },
        };
    }

    async fn send_message_to_server(
        &self,
        message: OutgoingMessage, 
    ) {
        match message.try_into() {
            Ok(message) => {
                if let Err(e) = self.sender.send(message).await {
                    eprintln!("Error sending message: {}", e);
                }
            }
            Err(e) => error!("Failed sending message to the server: {}", e),
        }
    }

    async fn reject_incomming_message(&mut self) {
        let error_message = format!("The node does not allow the request in this state: {:?}", self.state);
        self.send_message_to_server(ProtocolError::BadRequest(error_message).into()).await
    }
    
    
    
    async fn handle_successfull_authentication(&mut self) {
        self.change_state(State::Authenticated);
    }
    
    async fn handle_load_models(&mut self, models_to_load: LoadModels, task_id: String) {
        for model_to_load in models_to_load.model.into_iter() {
            let manager = match ModelManager::try_from(model_to_load) {
                Ok(mm) => mm,
                Err(e) => {
                    self.send_message_to_server(ProtocolError::UnableToLoadModel(e).into()).await;
                    continue;
                },
            };
            let config = manager.get_loaded_config();
            self.send_message_to_server(OutgoingMessage {
                message_type: OutgoingMessageType::ResponseLoadModel,
                task_id: task_id.clone(),
                body: OutgoingMessageBody::ResponseLoadModel(ResponseLoadModel {
                    handler_id: manager.get_id(),
                    config,
                }),
            }).await;
            self.models.insert(manager.get_id(), manager);
        }
    }
    
    async fn handle_embedding_request(&self, _message: SubmitEmbed) {
        todo!()
    }
    
    async fn handle_prompt_request(&self, _message: SubmitPrompt) {
        todo!()
    }

    fn change_state(&mut self, new_state: State) {
        info!("[Protocl manager] Changing state: {:?}", new_state);
        self.state = new_state;
    }
}
