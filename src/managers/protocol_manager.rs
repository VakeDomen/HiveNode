
use std::thread;

use dashmap::DashMap;
use log::{error, info};
use tokio::{runtime::Runtime, sync::mpsc::{self, Receiver, Sender}};

use crate::{llm::models::core::config::ModelConfig, managers::errors::ProtocolError, ws::messages::{message::{self, IncommingMessage, OutgoingMessage}, message_type::{IncommingMessageBody, IncommingMessageType, OutgoingMessageBody, OutgoingMessageType}, variants::{incomming::{load_models::LoadModels, submit_embed::SubmitEmbed, submit_prompt::SubmitPrompt}, outgoing::response_load_model::ResponseLoadModel}}};

use super::{manager::{Manager, State}, model_manager::ModelManager};


//  ___             __________              
// | C |    MSPC   |          |     MPSC    __________________
// | L |---------->|          |----------->| MODEL MANAGER 1 |  
// | I |           |          |<-----------|_________________|
// | E |           | PROTOCOL |     MPSC    __________________
// | N |           | MANAGER  |----------->| MODEL MANAGER 2 |               
// | T |           |          |<-----------|_________________|             
// |   |    MSPC   |          |     MPSC    __________________
// |   |<----------|          |----------->| MODEL MANAGER N |  
// |___|           |__________|<-----------|_________________|             
//                     
//                     



pub struct ProtocolManager {
    client_sender: Sender<OutgoingMessage>, // here we send messages to cliet to send to HiveCore
    client_reciever: Receiver<IncommingMessage>, // here we listen to messages send to client from HiveCore
    state: State,
    protocol_sender: Sender<OutgoingMessage>, // here we send messages to protocol managers (to pass to newly created managers)
    protocol_reciever: Receiver<OutgoingMessage>, // here we listen to messages from model managers
    models: DashMap<String, Sender<IncommingMessage>>
}

impl Manager for ProtocolManager {
    async fn start(&mut self) {
        if *self.get_state() != State::Offline {
            error!("Protocol manager already running.");
            return;
        }

        if !self.set_state(State::Unauthenticated) {
            return;
        }

        loop {
            tokio::select! {
                Some(message) = self.protocol_reciever.recv() => self.handle_outgoing_message(message).await,
                Some(message) = self.client_reciever.recv() => self.handle_incomming_message(message).await
            }
        }
    }

    fn get_state(&self) -> &State {
        &self.state
    }

    fn set_state(&mut self, new_state: State) -> bool {
        info!("[Protocl manager] Changing state: {:?}", new_state);
        self.state = new_state;
        true
    }

    fn get_reciever_mut(&mut self) -> &mut Receiver<IncommingMessage> {
        &mut self.client_reciever
    }

    fn get_sender_mut(&mut self) -> &mut Sender<OutgoingMessage> {
        &mut self.client_sender
    }

    // PROTOCOL
    async fn handle_incomming_message(&mut self, message: IncommingMessage) {
        let task_id = message.task_id.clone();
        match self.state {
            State::Offline => self.reject_incomming_message(message).await,
            State::Unauthenticated => match message.body {
                IncommingMessageBody::Success(_message) => self.handle_successfull_authentication().await,
                _ => self.reject_incomming_message(message).await,
            },
            State::Authenticated => match message.body {
                IncommingMessageBody::LoadModels(message) => self.handle_load_models(message, task_id).await,
                _ => self.reject_incomming_message(message).await,
            },
            State::Ready => match message.message_type {
                IncommingMessageType::SubmitEmbed => self.handle_embedding_request(message).await,
                IncommingMessageType::SubmitPrompt => self.handle_prompt_request(message).await,
                _ => self.reject_incomming_message(message).await,
                
                // IncommingMessageBody::SubmitEmbed(message) => 
                // IncommingMessageBody::SubmitPrompt(message) => 
            },
        };
    }
}

impl ProtocolManager {
    pub fn new(client_sender: Sender<OutgoingMessage>, client_reciever: Receiver<IncommingMessage>) -> Self {
        let (to_protocol_sender, to_protocol_reciever) = mpsc::channel::<OutgoingMessage>(100);
        Self {
            client_sender,
            client_reciever,
            state: State::Offline,
            models: DashMap::new(),
            protocol_sender: to_protocol_sender,
            protocol_reciever: to_protocol_reciever,
        }
    }

    

    async fn reject_incomming_message(&mut self, message: IncommingMessage) {
        let error_message = format!("The node does not allow the request in this state: {:?}", self.state);
        self.handle_outgoing_message(ProtocolError::BadRequest(error_message, message.task_id).into()).await
    }
    
    
    
    async fn handle_successfull_authentication(&mut self) {
        self.set_state(State::Authenticated);
    }
    
    async fn handle_load_models(&mut self, models_to_load: LoadModels, task_id: String) {
        
        for model_config in models_to_load.model.into_iter() {
            let (to_model_sender, to_model_reciever) = mpsc::channel::<IncommingMessage>(100);
            let model_config = match ModelConfig::try_from((model_config, task_id.clone())) {
                Ok(c) => c,
                Err(e) => return self.handle_outgoing_message(ProtocolError::UnableToLoadModel(e.into(), task_id).into()).await,
            };
        
            let task_id_movable = task_id.clone();
            let id = model_config.id.clone();
            let sender = self.protocol_sender.clone();
            let error_sender = self.protocol_sender.clone();
            let _ = thread::spawn(move || {
                let rt = Runtime::new().unwrap();
                let _ = rt.block_on(async move {
                    let mut manager = match ModelManager::try_from((
                        model_config,
                        sender,
                        to_model_reciever
                    )) {
                        Ok(mm) => mm,
                        Err(e) => {
                            return error_sender.send(ProtocolError::UnableToLoadModel(e.into(), task_id_movable).into()).await
                        },
                    };
                    manager.start().await;
                    Ok(())
                });
            });


            // let con
            self.models.insert(id, to_model_sender);

        }
        if !self.models.is_empty() {
            self.set_state(State::Ready);
        }
    }
    
    async fn handle_embedding_request(&self, _message: IncommingMessage) {
        todo!()
    }
    
    async fn handle_prompt_request(&mut self, message: IncommingMessage) {
        let body = match &message.body {
            IncommingMessageBody::SubmitPrompt(b) => b.clone(),
            _ => return,
        };
        let task_id = message.task_id.clone();
        let mut model_not_found = false;
        let mut cant_reach_model = false;
        {
            let model_handler = self.models.get_mut(&body.model_id);
        
            match model_handler {
                None => model_not_found = true,
                Some(m) => {
                    if let Err(_) = m.send(message).await {
                       cant_reach_model = true
                    };
                },
            } 
        } 
        if model_not_found {
           return self.handle_outgoing_message(ProtocolError::ModelNotFound(task_id.clone()).into()).await;
        } 

        if cant_reach_model {
            return self.handle_outgoing_message(ProtocolError::CantReachModel(task_id).into()).await;
        } 
    }

}
