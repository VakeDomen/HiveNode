pub mod incomming {
    pub mod submit_embed;
    pub mod submit_prompt;
    pub mod load_models;
}

pub mod outgoing {
    pub mod authentication;
    pub mod response_embed;
    pub mod response_prompt;
    pub mod response_prompt_token;
    pub mod response_load_model;
}

pub mod bidirectional {
    pub mod error;
    pub mod success;
}
