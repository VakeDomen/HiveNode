use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Error {
    pub code: u32,
    pub message: String,
}