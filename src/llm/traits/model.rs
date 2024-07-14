use super::{embedding::Embed, inferance::Infer, template::Template, tokenize::Tokenize};



pub trait LanguageModel: Tokenize + Template + Infer {
    fn prompt(&self, task: String) -> String;
}

pub trait EmbeddingModel: Tokenize + Embed {
    fn embed_text(&self, task: String) -> String;
}