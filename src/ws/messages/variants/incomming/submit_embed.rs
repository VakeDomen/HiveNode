use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubmitEmbed {
    pub model: String,
    pub polling: String,
    pub data: String,
}
