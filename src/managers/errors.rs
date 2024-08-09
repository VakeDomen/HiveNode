use crate::ws::messages::{message::OutgoingMessage, message_type::{OutgoingMessageBody, OutgoingMessageType}, variants::bidirectional::error::ErrorMessage};


pub enum ProtocolError {
    BadRequest(String),
    UnableToLoadModel(anyhow::Error),
    ModelNotFound,
}

impl From<ProtocolError> for OutgoingMessage {
    fn from(val: ProtocolError) -> Self {

        let (code, message) = match val {
            ProtocolError::BadRequest(message) => (0, message),
            ProtocolError::UnableToLoadModel(e) => (422, e.to_string()),
            ProtocolError::ModelNotFound => (404, "Model identifier not found".to_string()),
        };


        OutgoingMessage {
            message_type: OutgoingMessageType::Error,
            task_id: 0.to_string(),
            body: OutgoingMessageBody::Error(ErrorMessage {
                code,
                message,
            }),
        }
    }
}