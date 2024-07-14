use candle_core::quantized::gguf_file::Content;
use candle_transformers::models::quantized_llama::ModelWeights;
use anyhow::Result;
use tokenizers::Tokenizer;
use crate::llm::traits::{inferance::Infer, model::LanguageModel, tokenize::Tokenize};
use super::core::config::ModelConfig;
use super::core::token::Token;
use super::utils::loader::{load_gguf_content, load_tokenizer};


pub struct Llama3_8b {
    tokenizer: Tokenizer,
    weights: ModelWeights,
}

impl TryFrom<ModelConfig> for Llama3_8b {
    type Error = anyhow::Error;
    

    fn try_from(config: ModelConfig) -> Result<Self> {
        let tokenizer = load_tokenizer(config.tokenizer_path)?;
        let (mut file, content) = load_gguf_content(config.model_path)?;
        let weights = ModelWeights::from_gguf(content, &mut file, &config.device)?;

        Ok(Self {
            tokenizer,
            weights,
        })
    }    
}


impl Tokenize for Llama3_8b {
    fn tokenize(&self, data: String) -> Vec<candle_core::Tensor> {
        todo!()
    }
}

impl Infer for Llama3_8b {
    fn infer(&self, file_path: &Vec<Token>, device: &candle_core::Device) -> Result<Token> {
        todo!()
    }
}


impl LanguageModel for Llama3_8b {
    fn prompt(&self, task: String) -> String {
        todo!()
    }
}