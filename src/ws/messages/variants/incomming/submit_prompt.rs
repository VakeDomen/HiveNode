use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitPrompt {
    pub stream: bool,
    pub model: String,
    pub system_mesage: String,
    pub mode: String,
    pub history: Vec<String>,
    pub prompt: String,
}
