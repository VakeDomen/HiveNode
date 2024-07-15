use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitEmbed {
    pub model: String,
    pub polling: String,
    pub data: String,
}
