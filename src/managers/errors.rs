use crate::ws::messages::{message::OutgoingMessage, message_type::{OutgoingMessageBody, OutgoingMessageType}, variants::bidirectional::error::ErrorMessage};


pub enum ProtocolError {
    BadRequest(String),
}



impl Into<OutgoingMessage> for ProtocolError {
    fn into(self) -> OutgoingMessage {

        let (code, message) = match self {
            ProtocolError::BadRequest(message) => (0, message),
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