use candle_core::{Device, Tensor};
use candle_transformers::models::quantized_llama::ModelWeights;
use anyhow::Result;
use tokenizers::Tokenizer;
use crate::llm::traits::template::{Template, TemplatedPrompt, Eos};
use crate::llm::traits::{inferance::Infer, model::LanguageModel, tokenize::Tokenize};
use super::core::config::ModelConfig;
use super::utils::loader::{load_gguf_content, load_tokenizer};


pub struct Llama3_8b {
    tokenizer: Tokenizer,
    model: ModelWeights,
    device: Device,
    max_seq_len: usize,
    max_sample_len: usize,
}

impl TryFrom<ModelConfig> for Llama3_8b {
    type Error = anyhow::Error;
    
    fn try_from(config: ModelConfig) -> Result<Self> {
        let tokenizer = load_tokenizer(config.tokenizer_path)?;
        let (mut file, content) = load_gguf_content(config.model_path)?;
        let model = ModelWeights::from_gguf(content, &mut file, &config.device)?;
        let device = config.device;
        let max_seq_len = config.max_seq_len;
        let max_sample_len = config.max_sample_len;
        Ok(Self {
            tokenizer,
            model,
            device,
            max_seq_len,
            max_sample_len,
        })
    }    
}

impl Template for Llama3_8b {
    fn prompt_template(&self, system_msg: &String, user_message: &String) -> TemplatedPrompt {
        format!(
            "<|begin_of_text|><|start_header_id|>system<|end_header_id|>\n\n{}<|eot_id|><|start_header_id|>user<|end_header_id|>\n\n{}<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n\n", 
            system_msg,
            user_message,
        )
    }
    
    fn get_eos(&self) -> Eos {
        "<|eot_id|>".to_owned()
    }
}


impl Tokenize for Llama3_8b {
    fn tokenizer(&self) -> &Tokenizer {
        &self.tokenizer
    }
}

impl Infer for Llama3_8b {

    
    fn get_max_sample_len(&self) -> usize {
        self.max_sample_len
    }

    fn get_max_sequence_len(&self) -> usize {
        self.max_seq_len
    }

    fn get_device(&self) -> &Device {
        &self.device
    }

    fn forward(&mut self, input: &Tensor, position: usize) -> Result<Tensor> {
        Ok(self.model.forward(input, position)?)
    }
    
    fn get_model_name(&self) -> String {
        "Llama3_8b".to_string()
    }
}


impl LanguageModel for Llama3_8b {}