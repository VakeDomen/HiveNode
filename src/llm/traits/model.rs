use crate::{llm::models::{core::config::{ModelConfig, ModelIdentifier}, llama3_8b::Llama3_8b}, managers::errors::ModelManagerError, ws::messages::message::IncommingMessage};

use super::{embedding::Embed, inferance::Infer, template::Template, tokenize::Tokenize};
use anyhow::{anyhow, Result};


pub trait LanguageModel: Tokenize + Template + Infer {
    fn prompt(&mut self, task: IncommingMessage) -> Result<String>;
}

pub trait EmbeddingModel: Tokenize + Embed {
    fn embed_text(&mut self, task: IncommingMessage) -> Result<String>;
}

pub enum Model {
    EmbeddingModel(Box<dyn EmbeddingModel>),
    LanguageModel(Box<dyn LanguageModel>),
}
impl Model {
    pub fn prompt(&mut self, message: IncommingMessage) -> Result<String> {
        match self {
            Model::LanguageModel(model) => model.prompt(message),
            _ => return Err(anyhow!(ModelManagerError::InvalidModelAction(message.task_id))),
        }
    }
    pub fn embed(&mut self, message: IncommingMessage) -> Result<String> {
        match self {
            Model::EmbeddingModel(model) => model.embed_text(message),
            _ => return Err(anyhow!(ModelManagerError::InvalidModelAction(message.task_id))),
        }
    }
}




impl TryFrom<ModelConfig> for Model {
    type Error = anyhow::Error;

    fn try_from(model_config: ModelConfig) -> Result<Self> {
        let identifier = ModelIdentifier::try_from(&model_config.model_name)?;
        match identifier {
            ModelIdentifier::Llama3_8b => Ok(Model::LanguageModel(Box::new(Llama3_8b::try_from(model_config)?))),
        }
    }
}