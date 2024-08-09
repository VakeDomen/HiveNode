use crate::{llm::models::{core::config::{ModelConfig, ModelIdentifier}, llama3_8b::Llama3_8b}, managers::errors::ModelManagerError, ws::messages::{message::{IncommingMessage, OutgoingMessage}, message_type::IncommingMessageBody}};

use super::{embedding::Embed, inferance::Infer, template::Template, tokenize::Tokenize};
use anyhow::{anyhow, Result};
use tokio::sync::mpsc::Sender;


pub enum Model {
    EmbeddingModel(Box<dyn EmbeddingModel>),
    LanguageModel(Box<dyn LanguageModel>),
}
impl Model {
    pub fn prompt(&mut self, message: IncommingMessage, sender: Sender<OutgoingMessage>) -> Result<()> {
        match self {
            Model::LanguageModel(model) => model.prompt(message, sender),
            _ => return Err(anyhow!(ModelManagerError::InvalidModelAction(message.task_id))),
        }
    }
    pub fn embed(&mut self, message: IncommingMessage, sender:  Sender<OutgoingMessage>) -> Result<String> {
        match self {
            Model::EmbeddingModel(model) => model.embed_text(message, sender),
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
pub trait LanguageModel: Tokenize + Template + Infer {
    fn prompt(&mut self, task: IncommingMessage, sender: Sender<OutgoingMessage>) -> Result<()> {
        let body = match task.body {
            IncommingMessageBody::SubmitPrompt(b) => b,
            _ => return Err(ModelManagerError::InvalidModelAction(task.task_id).into()),
        };

        let prompt = self.prompt_template(&body.system_mesage, &body.prompt);
        let tokenized_prompt = self.tokenize(prompt)?;
        let response = self.infer(&tokenized_prompt, sender.clone(), task.task_id.clone())?;
        self.send_response(task.task_id, body, response, sender);
        Ok(())
    }
}

pub trait EmbeddingModel: Tokenize + Embed {
    fn embed_text(&mut self, task: IncommingMessage, sender: Sender<OutgoingMessage>) -> Result<String>;
}


