use std::env::VarError;
use std::fmt::{Display, Formatter};

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

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}
