use serde::{Deserialize, Serialize};
use crate::ws::messages::variants::{
    incomming::{submit_embed::SubmitEmbed, submit_prompt::SubmitPrompt},
    outgoing::{
        authentication::Authentication, response_embed::ResponseEmbed,
        response_prompt::ResponsePrompt, response_prompt_token::ResponsePromptToken,
    },
    bidirectional::error::ErrorMessage,
};

use super::variants::incomming::load_models::LoadModels;


#[derive(Debug, Deserialize)]
pub enum IncommingMessageType {
    LoadModels,
    SubmitEmbed,
    SubmitPrompt,
    Error,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IncommingMessageBody {
    LoadModels(LoadModels),
    SubmitEmbed(SubmitEmbed),
    SubmitPrompt(SubmitPrompt),
    Error(ErrorMessage),
}


#[derive(Debug, Serialize)]
pub enum OutgoingMessageType {
    Authentication,
    ResponseEmbed,
    ResponsePrompt,
    ResponsePromptToken,
    Error,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OutgoingMessageBody {
    Authentication(Authentication),
    ResponseEmbed(ResponseEmbed),
    ResponsePrompt(ResponsePrompt),
    ResponsePromptToken(ResponsePromptToken),
    Error(ErrorMessage),
}
