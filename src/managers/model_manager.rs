
use crate::{llm::{models::core::config::{ModelConfig, ModelConfigPublic}, traits::model::Model}, ws::messages::{message::{IncommingMessage, OutgoingMessage}, variants::incomming::load_models::RequestModelConfig}};
use anyhow::Result;
use log::{error, info};
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;
use manager::State;
use super::manager::{self, Manager};

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
        // fig = manager.get_loaded_config();
            // self.send_message(OutgoingMessage {
            //     message_type: OutgoingMessageType::ResponseLoadModel,
            //     task_id: task_id.clone(),
            //     body: OutgoingMessageBody::ResponseLoadModel(ResponseLoadModel {
            //         handler_id: manager.get_id(),
            //         config,
            //     }),
            // }).await;

        self.set_state(State::Ready);
        loop {
            
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
        todo!()
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
}