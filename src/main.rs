use std::env;

use tokio_tungstenite::tungstenite::{connect, Message};
use dotenv::dotenv;

fn main() {
    dotenv().ok();

    let url = env::var("HIVE_CORE_URL").expect("Enviroment variable HIVE_CORE_URL not set.");
    let (mut socket, response) = connect(url).expect("Can't connect");

    println!("Connected to the server");
    println!("Response HTTP code: {}", response.status());
    println!("Response contains the following headers:");
    for (ref header, value) in response.headers() {
        println!("* {}: {:?}", header, value);
    }

    socket.send(Message::Text("{\"type\": \"Authentication\",\"body\": {\"token\": \"token\", \"HW\": [{ \"GPU_model\": \"nvidia 1080ti\", \"GPU_VRAM\": 800, \"driver\": \"vidia.476\", \"CUDA\": \"9.2\"},{ \"GPU_model\": \"nvidia 1080ti\", \"GPU_VRAM\": 800, \"driver\": \"vidia.476\", \"CUDA\": \"9.2\"} ]} }".into())).unwrap();
    loop {
        let msg = socket.read().expect("Error reading message");
        println!("*********** Received ***********");
        println!("HEADERS:");        
        for (ref header, value) in response.headers() {
            println!("* {}: {:?}", header, value);
        }
        println!("BODY:");
        println!("{msg}");
        println!("********************************");
        println!();
        println!();
    }
    // socket.close(None);

}
