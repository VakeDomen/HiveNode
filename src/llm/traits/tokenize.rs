use anyhow::Result;
use tokenizers::Tokenizer;
use crate::llm::models::core::token::Token;

pub trait Tokenize {
    fn tokenizer(&self) -> &Tokenizer;
    fn tokenize(&self, data: String) -> Result<Vec<Token>> {
        let encoding = self
            .tokenizer()
            .encode(data, true)
            .map_err(anyhow::Error::msg)?;
        Ok(encoding.get_ids().to_vec())
    }
}