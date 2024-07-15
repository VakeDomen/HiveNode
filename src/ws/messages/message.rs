use serde::{Deserialize, Serialize};

use super::message_type::{IncommingMessageBody, IncommingMessageType, OutgoingMessageBody, OutgoingMessageType};


#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IncommingMessage {
    
    #[serde(rename = "type")]
    message_type: IncommingMessageType,
    
    #[serde(rename = "taskId")]
    task_id: String,
    
    #[serde(flatten)]
    body: IncommingMessageBody

}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OutgoingMessage {
    
    #[serde(rename = "type")]
    message_type: OutgoingMessageType,
    
    task_id: String,
    
    #[serde(flatten)]
    body: OutgoingMessageBody
}