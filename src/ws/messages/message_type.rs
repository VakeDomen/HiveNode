use serde::{Deserialize, Serialize};
use crate::ws::messages::variants::{
    incomming::{submit_embed::SubmitEmbed, submit_prompt::SubmitPrompt},
    outgoing::{
        authentication::Authentication, response_embed::ResponseEmbed,
        response_prompt::ResponsePrompt, response_prompt_token::ResponsePromptToken,
    },
    bidirectional::error::ErrorMessage,
};
use super::variants::{bidirectional::success::SuccessMessage, incomming::load_models::LoadModels, outgoing::response_load_model::ResponseLoadModel};


#[derive(Debug, Deserialize, PartialEq, Clone)]
pub enum IncommingMessageType {
    Success,
    LoadModels,
    SubmitEmbed,
    SubmitPrompt,
    Error,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase", untagged)]
pub enum IncommingMessageBody {
    Success(SuccessMessage),
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
    ResponseLoadModel,
    Error,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OutgoingMessageBody {
    Authentication(Authentication),
    ResponseLoadModel(ResponseLoadModel),
    ResponseEmbed(ResponseEmbed),
    ResponsePrompt(ResponsePrompt),
    ResponsePromptToken(ResponsePromptToken),
    Error(ErrorMessage),
}
