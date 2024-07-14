use anyhow::Result;
use candle_core::Device;

use crate::llm::models::core::token::Token;


pub trait Infer {
    fn infer(&self, tokens: &Vec<Token>, device: &Device) -> Result<Token>;
}


