use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct ResponsePromptToken {
    pub model: String,
    pub token: String,
}
