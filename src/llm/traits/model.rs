use crate::llm::models::{core::config::{ModelConfig, ModelIdentifier}, llama3_8b::Llama3_8b};

use super::{embedding::Embed, inferance::Infer, template::Template, tokenize::Tokenize};
use anyhow::Result;

pub enum Model {
    EmbeddingModel(Box<dyn EmbeddingModel>),
    LanguageModel(Box<dyn LanguageModel>),
}

pub trait LanguageModel: Tokenize + Template + Infer {
    fn prompt(&self, task: String) -> String;
}

pub trait EmbeddingModel: Tokenize + Embed {
    fn embed_text(&self, task: String) -> String;
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