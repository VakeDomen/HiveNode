use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct Authentication {
    pub token: String,
    #[serde(rename = "HW")]
    pub hardware: Vec<GPU>,
}

#[derive(Debug, Default, Serialize)]
pub struct GPU {

    #[serde(rename = "GPU_model")]
    pub gpu_model: String,

    #[serde(rename = "GPU_VRAM")]
    pub gpu_vram: u32,

    pub driver: String,

    #[serde(rename = "CUDA")]
    pub cuda: String,
}