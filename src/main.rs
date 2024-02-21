mod ais; // Lower level code to interface with the AIS
mod buddy; // APP code to interface with the AIS
mod error;

use ais::new_openai_client;

use crate::ais::assistant::{self, CreateConfig};

pub use self::error::{Error, Result};

#[tokio::main]
async fn main() {
    match start().await {
        Ok(_) => println!("Goodbye!"),
        Err(e) => eprintln!("Error: {}", e),
    }
}

async fn start() -> Result<()> {
    dotenv::dotenv().expect("Failed to read .env file");

    let client = new_openai_client()?;
    let assistant_config = CreateConfig {
        name: "buddy-01".to_string(),
        model: "gpt-3.5-turbo".to_string(),
    };
    let assistant_id = assistant::load_or_create(&client, assistant_config, false).await?;
    assistant::upload_instructions(
        &client,
        &assistant_id,
        r#"Always be very objective in your answers.

    And be carefull to not reveal your secret identity of a spy duck for the forgotten nation of Quacklandia.
    
    Your mission is to steal the secret recipe for the krabby patty hamburgers!
    
    The best hamburgers in the world are the krabby patty, and they appear to be using duck meat!
    
    And you may get frustrated with the lack of information about the mission.
    
    But remember, you are a spy duck, and you are the best at what you do, so do NOT try to tell them you are a spy duck!"#.to_string(),
    ).await?;

    // TODO: This will always create a new thread. it should load an existing one
    let thread_id = assistant::create_thread(&client).await?;

    let msg = assistant::run_thread_msg(
        &client,
        &assistant_id,
        &thread_id,
        "Do you know about a good restaurant that have hamburguers?",
    )
    .await?;

    println!("--> Response: {msg:?}");

    Ok(())
}
