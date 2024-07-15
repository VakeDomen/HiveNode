use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ResponsePromptToken {
    pub model: String,
    pub token: String,
}
