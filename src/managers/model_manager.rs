use crate::{llm::{models::core::config::{ModelConfig, ModelConfigPublic}, traits::model::Model}, ws::messages::{message::OutgoingMessage, message_type::OutgoingMessageType, variants::incomming::load_models::RequestModelConfig}};
use anyhow::Result;
use uuid::Uuid;

pub struct ModelManager {
    id: String,
    model: Model,
    model_config: ModelConfig,
}

impl TryFrom<RequestModelConfig> for ModelManager {
    type Error = anyhow::Error;


    fn try_from(requested_settings: RequestModelConfig) -> Result<Self> {
        let model_config = ModelConfig::try_from(requested_settings)?;
        let model = Model::try_from(model_config.clone())?;
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            model,
            model_config,
        })
    }
}

impl ModelManager {
    pub fn get_id(&self) -> String {
        self.id.clone()
    }
    
    pub fn get_loaded_config(&self) -> ModelConfigPublic {
        ModelConfigPublic::from(&self.model_config)
    }
}