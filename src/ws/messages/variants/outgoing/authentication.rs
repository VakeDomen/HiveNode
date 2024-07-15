use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Authentication {
    pub token: String,
    #[serde(rename = "HW")]
    pub hardware: Vec<GPU>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GPU {

    #[serde(rename = "GPU_model")]
    pub gpu_model: String,

    #[serde(rename = "GPU_VRAM")]
    pub gpu_vram: u32,

    pub driver: String,

    #[serde(rename = "CUDA")]
    pub cuda: String,
}