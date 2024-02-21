pub mod assistant;
pub mod msg;

use async_openai::{config::OpenAIConfig, Client};

use crate::Result;

pub type OpenAIClient = Client<OpenAIConfig>;

pub fn new_openai_client() -> Result<OpenAIClient> {
    if dotenv::var("OPENAI_API_KEY").is_ok() {
        Ok(Client::new())
    } else {
        println!("No OPENAI_API_KEY variable in .env. Please add it and try again.");
        Err("No OPENAI_API_KEY in .env".into())
    }
}
