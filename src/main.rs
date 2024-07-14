use candle_transformers::models::quantized_llama::MAX_SEQ_LEN;
use dotenv::dotenv;
use llm::{models::{core::config::ModelConfig, llama3_8b::Llama3_8b, utils::loader::load_device}, traits::{inferance::Infer, tokenize::Tokenize}};
use logging::logger::init_logging;
use ws::client::connect_to_hive;
use anyhow::Result;

mod ws;
mod config;
mod logging;
mod llm;
mod protocol;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_logging()?;

    let llama3_config = ModelConfig { 
        model_path: "./models/llama3-8b/Meta-Llama-3-8B-Instruct.Q5_K_M.gguf".into(), 
        tokenizer_path: "./models/llama3-8b/tokenizer.json".into(), 
        device: load_device(Some(0)), 
        max_seq_len: MAX_SEQ_LEN, 
        max_sample_len: 1000 
    };


    let mut model = Llama3_8b::try_from(llama3_config)?;

    let prompt = model.tokenize("What is your purpouse?".into())?;
    
    let resp = model.infer(&prompt)?;
    println!("{resp}");
    let _ = connect_to_hive();
    Ok(())
}
