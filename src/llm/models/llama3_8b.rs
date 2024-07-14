use candle_core::quantized::gguf_file::Content;
use candle_transformers::models::quantized_llama::ModelWeights;
use anyhow::Result;
use crate::llm::traits::loading::Load;


struct Llama3_8b;

impl Load for Llama3_8b {
    fn load(file_path: &str, device: &candle_core::Device) -> Result<candle_transformers::models::quantized_llama::ModelWeights> {
        let model_path = std::path::PathBuf::from(file_path);
        let mut file = std::fs::File::open(&model_path)?;
        let model = Content::read(&mut file).map_err(|e| e.with_path(file_path))?;
        let weights = ModelWeights::from_gguf(model, &mut file, device)?;
        Ok(weights)
    }
}