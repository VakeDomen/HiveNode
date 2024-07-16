use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadModels {
    pub model: Vec<RequestModelSettings>,
}


#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestModelSettings {
    model_name: String,
    device: usize,
    max_seq_len: usize,
}