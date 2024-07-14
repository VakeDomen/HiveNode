use anyhow::Result;
use candle_core::Device;
use candle_transformers::models::quantized_llama::ModelWeights;


pub trait Load {
    fn load(file_path: &str, device: &Device) -> Result<ModelWeights>;
}


