use std::fmt;
use crate::ws::messages::{message::OutgoingMessage, message_type::{OutgoingMessageBody, OutgoingMessageType}, variants::bidirectional::error::ErrorMessage};


type TaskId = String;

#[derive(Debug)]
pub enum ModelManagerError {
    ModelNotReady(TaskId),
    InvalidModelAction(TaskId),
}

impl std::error::Error for ModelManagerError {}

impl fmt::Display for ModelManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelManagerError::ModelNotReady(task_id) => write!(f, "[TASK: {task_id}] Model is not ready for action."),
            ModelManagerError::InvalidModelAction(task_id) => write!(f, "[TASK: {task_id}] This action is not available for the chosen model."),
        }
    }
}

impl From<ModelManagerError> for OutgoingMessage {
    fn from(val: ModelManagerError) -> Self {

        let (code, message) = match val {
            ModelManagerError::ModelNotReady(task_id) => (500, "Model is not ready for action.".into()),
            ModelManagerError::InvalidModelAction(task_id) => (300, "This action is not avalible for the chosen model".into()),
            
        };

        OutgoingMessage {
            message_type: OutgoingMessageType::Error,
            task_id: code.to_string(),
            body: OutgoingMessageBody::Error(ErrorMessage {
                code,
                message,
            }),
        }
    }
}



#[derive(Debug)]
pub enum ProtocolError {
    BadRequest(String, TaskId),
    UnableToLoadModel(anyhow::Error, TaskId),
    ModelNotFound(TaskId),
    CantReachModel(TaskId),
}

impl std::error::Error for ProtocolError {}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolError::BadRequest(message, task_id) => write!(f, "[TASK: {task_id}] Bad request: {}", message),
            ProtocolError::UnableToLoadModel(e, task_id) => write!(f, "[TASK: {task_id}] Unable to load model: {}", e),
            ProtocolError::ModelNotFound(task_id) => write!(f, "[TASK: {task_id}] Model identifier not found"),
            ProtocolError::CantReachModel(task_id) => write!(f, "[TASK: {task_id}] Can't reach model"),
        }
    }
}

impl From<ProtocolError> for OutgoingMessage {
    fn from(val: ProtocolError) -> Self {

        let (code, message, task_id) = match val {
            ProtocolError::BadRequest(message, task_id) => (300, message, task_id),
            ProtocolError::UnableToLoadModel(e, task_id) => (422, e.to_string(), task_id),
            ProtocolError::ModelNotFound(task_id) => (404, "Model identifier not found".to_string(), task_id),
            ProtocolError::CantReachModel(task_id) => (500, "Can't reach model".to_string(), task_id),
            
        };

        OutgoingMessage {
            message_type: OutgoingMessageType::Error,
            task_id: task_id,
            body: OutgoingMessageBody::Error(ErrorMessage {
                code,
                message,
            }),
        }
    }
}