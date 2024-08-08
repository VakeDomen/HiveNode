use serde::Serialize;

use crate::llm::models::core::config::ModelConfigPublic;

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseLoadModel {
    pub handler_id: String,
    pub config: ModelConfigPublic,
}