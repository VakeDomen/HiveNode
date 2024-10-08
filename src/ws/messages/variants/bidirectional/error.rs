use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ErrorMessage {
    pub code: u32,
    pub message: String,
}