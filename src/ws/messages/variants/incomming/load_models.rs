use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadModels {
    pub model: Vec<RequestModelConfig>,
}


#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestModelConfig {
    pub model_name: String,
    pub device: usize,
    pub max_sample_len: usize,
}