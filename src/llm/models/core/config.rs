use candle_core::Device;
use anyhow::Result;
use candle_transformers::models::quantized_llama::MAX_SEQ_LEN;
use serde::Serialize;
use uuid::Uuid;
use crate::{llm::models::utils::loader::load_device, ws::messages::variants::incomming::load_models::RequestModelConfig};




#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub id: String,
    pub model_name: String,
    pub model_path: String,
    pub tokenizer_path: String,
    pub device: Device,
    pub max_seq_len: usize,
    pub max_sample_len: usize,
}

// data that will be reported to the server
#[derive(Debug, Default, Serialize)]
pub struct ModelConfigPublic {
    pub model_name: String,
    pub max_seq_len: usize,
    pub max_sample_len: usize,
}

pub enum ModelIdentifier {
    Llama3_8b,
}

impl TryFrom<&String> for ModelIdentifier {
    type Error = anyhow::Error;

    fn try_from(name: &String) -> Result<Self> {
        match name.as_str() {
            "llama3_8b" => Ok(ModelIdentifier::Llama3_8b),
            _ => Err(anyhow::anyhow!("Unknown model identifier: {:?}", name)),
        }
    }
}

impl From<&ModelConfig> for ModelConfigPublic {
    fn from(conf: &ModelConfig) -> Self {
        Self {
            model_name: conf.model_name.clone(),
            max_seq_len: conf.max_seq_len,
            max_sample_len: conf.max_seq_len,
        }
    }
}

impl TryFrom<RequestModelConfig> for ModelConfig {
    type Error = anyhow::Error;

    fn try_from(settings: RequestModelConfig) -> Result<Self> {
        let RequestModelConfig { model_name, device, max_sample_len  } = settings;
        let model = ModelIdentifier::try_from(&model_name)?;
        let (model_path, tokenizer_path) = get_model_file_paths(&model);
        let max_seq_len = get_model_max_seq_len(&model);
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            model_name,
            model_path,
            tokenizer_path,
            device: load_device(Some(device)),
            max_sample_len,
            max_seq_len,
        })
    }
}

fn get_model_file_paths(model: &ModelIdentifier) -> (String, String) {
    match model {
        ModelIdentifier::Llama3_8b => (
            "./resources/llama3-8b/Meta-Llama-3-8B-Instruct.Q5_K_M.gguf".into(),
            "./resources/llama3-8b/tokenizer.json".into(),
        )
    }
}

fn get_model_max_seq_len(model: &ModelIdentifier) -> usize {
    match model {
        ModelIdentifier::Llama3_8b => MAX_SEQ_LEN,
    }
}