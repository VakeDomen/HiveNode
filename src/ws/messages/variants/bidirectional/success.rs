use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SuccessMessage {
    pub code: u32,
    pub message: String,
}