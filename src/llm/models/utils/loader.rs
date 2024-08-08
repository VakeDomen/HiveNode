use std::fs::File;

use anyhow::{Error, Result};
use candle_core::{quantized::gguf_file::Content, Device};
use log::error;
use tokenizers::Tokenizer;

pub fn load_tokenizer(path: String) -> Result<Tokenizer> {
    let tokenizer_path = std::path::PathBuf::from(path);
    Tokenizer::from_file(tokenizer_path).map_err(Error::msg)
}

pub fn load_gguf_content(path: String) -> Result<(File, Content)> {
    let mut file = std::fs::File::open(&path)?;
    let content = Content::read(&mut file).map_err(|e| e.with_path(path))?;
    Ok((file, content))
}

pub fn load_device(gpu_id: Option<usize>) -> Device {
    if let Some(id) = gpu_id {
        match Device::new_cuda(id) {
            Ok(cuda) => cuda,
            Err(e) => {
                error!("Error initializing CUDA device. Switching to CPU. Error: {:#?}", e);
                Device::Cpu
            },
        }
    } else {
        Device::Cpu
    }
}