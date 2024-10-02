use reqwest::blocking::Response;
use std::convert::TryFrom;
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Deserialize, Serialize)]
pub struct ModelDetails {
    pub parent_model: String,
    pub format: String,
    pub family: String,
    pub families: Vec<String>,
    pub parameter_size: String,
    pub quantization_level: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Model {
    pub name: String,
    pub model: String,
    pub modified_at: String,
    pub size: u64,
    pub digest: String,
    pub details: ModelDetails,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Tags {
    pub models: Vec<Model>,
}

impl TryFrom<Response> for Tags {
    type Error = anyhow::Error;

    fn try_from(response: Response) -> Result<Self> {
        let body = response.text()?;
        let models_response: Tags = serde_json::from_str(&body)?;
        Ok(models_response)
    }
}

