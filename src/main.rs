use dotenv::dotenv;
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
    let _ = connect_to_hive();
    Ok(())
}
