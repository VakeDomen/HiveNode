use serde::Serialize;

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseEmbed {
    pub model: String,
    pub polling: String,
    pub embedding_vector: Vec<f32>,
    pub tokenizer_time: u64,
    pub tokens_processed: u32,
}