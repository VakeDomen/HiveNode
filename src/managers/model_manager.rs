
use crate::{llm::{models::core::config::{ModelConfig, ModelConfigPublic}, traits::model::Model}, ws::messages::{message::{IncommingMessage, OutgoingMessage}, message_type::{IncommingMessageType, OutgoingMessageBody, OutgoingMessageType}, variants::{incomming::load_models::RequestModelConfig, outgoing::response_load_model::ResponseLoadModel}}};
use anyhow::Result;
use log::{error, info};
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;
use manager::State;
use super::{errors::{ModelManagerError, ProtocolError}, manager::{self, Manager}};

pub struct ModelManager {
    state: State,
    sender: Sender<OutgoingMessage>,
    reciever: Receiver<IncommingMessage>,
    model: Model,
    model_config: ModelConfig,
}

impl TryFrom<(ModelConfig, Sender<OutgoingMessage>, Receiver<IncommingMessage>)> for ModelManager {
    type Error = anyhow::Error;


    fn try_from(settings: (ModelConfig, Sender<OutgoingMessage>, Receiver<IncommingMessage>)) -> Result<Self> {
        let ( model_config, sender, reciever ) = settings;
        let model = Model::try_from(model_config.clone())?;
        Ok(Self {
            state: State::Offline,
            sender,
            reciever,
            model,
            model_config,
        })
    }
}

impl Manager for ModelManager {
    async fn start(&mut self) {
        if *self.get_state() != State::Offline {
            error!("Protocol manager already running.");
            return;
        }

        if !self.set_state(State::Ready) {
            return;
        }
        
        self.handle_outgoing_message(OutgoingMessage {
            message_type: OutgoingMessageType::ResponseLoadModel,
            task_id: self.model_config.request_packet_id.clone(),
            body: OutgoingMessageBody::ResponseLoadModel(ResponseLoadModel {
                handler_id: self.get_id(),
                config: self.get_loaded_config(),
            }),
        }).await;

        
        loop {
            tokio::select! {
                Some(message) = self.get_reciever_mut().recv() => self.handle_incomming_message(message).await,
            } 
        }
    }

    fn get_state(&self) -> &State {
        &self.state
    }

    fn set_state(&mut self, new_state: State) -> bool {
        info!("[Model manager | {}] Changing state: {:?}", self.get_id(), new_state);
        self.state = new_state;
        true
    }

    fn get_reciever_mut(&mut self) -> &mut Receiver<IncommingMessage> {
        &mut self.reciever
    }

    fn get_sender_mut(&mut self) -> &mut Sender<OutgoingMessage> {
        &mut self.sender
    }

    async fn handle_incomming_message(&mut self, message: IncommingMessage) {
        if *self.get_state() != State::Ready {
            self.handle_outgoing_message(ModelManagerError::ModelNotReady(message.task_id).into()).await;
            return ;
        }

        let sender = self.get_sender_mut().clone();
        let _  = match message.message_type {
            IncommingMessageType::SubmitEmbed => todo!(),
            IncommingMessageType::SubmitPrompt => self.model.prompt(message, sender),
            _ => return self.reject_message(message).await,
        };
        
    }
}


impl ModelManager {

    pub fn get_id(&self) -> String {
        self.model_config.id.clone()
    }
    
    pub fn get_loaded_config(&self) -> ModelConfigPublic {
        ModelConfigPublic::from(&self.model_config)
    }

    // pub fn infer
    
    // pub fn prompt(&self, ) {
    //     todo!()
    // }

    fn change_state(&mut self, new_state: State) {
        info!("[Model manager] Changing state: {:?}", new_state);
        self.state = new_state;
    }
    
    async fn reject_message(&mut self, message: IncommingMessage) {
        let error_message = format!("The model manager does not allow the request in this state: {:?}", self.state);
        self.handle_outgoing_message(ProtocolError::BadRequest(error_message, message.task_id).into()).await
    }
}