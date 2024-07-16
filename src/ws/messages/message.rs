use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;
use anyhow::Result;
use super::{message_type::{IncommingMessageBody, IncommingMessageType, OutgoingMessageBody, OutgoingMessageType}, variants::{self, bidirectional::error::ErrorMessage, outgoing::authentication::Authentication}};


#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncommingMessage {
    
    #[serde(rename = "type")]
    pub message_type: IncommingMessageType,
    
    #[serde(rename = "taskId")]
    pub task_id: String,
    
    #[serde(flatten)]
    pub body: IncommingMessageBody

}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutgoingMessage {
    
    #[serde(rename = "type")]
    pub message_type: OutgoingMessageType,
    
    pub task_id: String,
    
    #[serde(flatten)]
    pub body: OutgoingMessageBody
}


impl TryInto<Message> for OutgoingMessage {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Message> {
        Ok(Message::Text(serde_json::to_string(&self)?))
    }
}

impl Default for OutgoingMessage {
    fn default() -> Self {
        Self { 
            message_type: OutgoingMessageType::Authentication, 
            task_id: Default::default(), 
            body: OutgoingMessageBody::Authentication(Authentication::default()) 
        }
    }
}