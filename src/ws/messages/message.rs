use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;
use anyhow::Result;
use super::message_type::{
    IncommingMessageBody, 
    IncommingMessageType, 
    OutgoingMessageBody, 
    OutgoingMessageType
};


#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IncommingMessage {
    
    #[serde(rename = "type")]
    pub message_type: IncommingMessageType,
    
    #[serde(rename = "taskId")]
    pub task_id: String,
    
    #[serde(rename = "body")]
    // #[serde(flatten)]
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