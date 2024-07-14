use super::{embedding::Embed, inferance::Infer, tokenize::Tokenize};



pub trait LanguageModel: Tokenize + Infer {
    fn prompt(&self, task: String) -> String;
}

pub trait EmbeddingModel: Tokenize + Embed {
    fn embed_text(&self, task: String) -> String;
}