use serde::{Deserialize, Serialize};
use crate::ws::messages::variants::{
    incomming::{submit_embed::SubmitEmbed, submit_prompt::SubmitPrompt},
    outgoing::{
        authentication::Authentication, response_embed::ResponseEmbed,
        response_prompt::ResponsePrompt, response_prompt_token::ResponsePromptToken,
    },
    bidirectional::error::Error,
};


#[derive(Debug, Deserialize)]
pub enum IncommingMessageType {
    SubmitEmbed,
    SubmitPrompt,
    Error,
}


#[derive(Debug, Serialize)]
pub enum OutgoingMessageType {
    Authenitcation,
    ResponseEmbed,
    ResponsePrompt,
    ResponsePromptToken,
    Error,
}


#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IncommingMessageBody {
    SubmitEmbed(SubmitEmbed),
    SubmitPrompt(SubmitPrompt),
    Error(Error),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OutgoingMessageBody {
    Authentication(Authentication),
    ResponseEmbed(ResponseEmbed),
    ResponsePrompt(ResponsePrompt),
    ResponsePromptToken(ResponsePromptToken),
    Error(Error),
}
