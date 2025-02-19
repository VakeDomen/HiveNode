use std::env::VarError;

use nvml_wrapper::error::NvmlError;

#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl From<VarError> for Error {
    fn from(value: VarError) -> Self {
        Self {
            message: format!("{:?}", value),
        }
    }
}
impl From<NvmlError> for Error {
    fn from(value: NvmlError) -> Self {
        Self {
            message: format!("{:?}", value),
        }
    }
}
