use serde::Serialize;

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponsePrompt {
    pub model: String,
    pub system_mesage: String,
    pub mode: String,
    pub response: String,
    pub tokenizer_time: u64,
    pub inference_time: u64,
    pub tokens_processed: u32,
    pub tokens_generated: u64,
}