use dotenv::dotenv;
use log::error;
use logging::logger::init_logging;
use ws::client::connect_to_hive;
use anyhow::Result;

mod ws;
mod config;
mod logging;
mod llm;
mod managers;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_logging()?;

    if let Err(e) = connect_to_hive().await {
        error!("Server error: {:#?}", e);
    }
    Ok(())
}
