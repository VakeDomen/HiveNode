pub mod incomming {
    pub mod submit_embed;
    pub mod submit_prompt;
}

pub mod outgoing {
    pub mod authentication;
    pub mod response_embed;
    pub mod response_prompt;
    pub mod response_prompt_token;
}

pub mod bidirectional {
    pub mod error;
}
