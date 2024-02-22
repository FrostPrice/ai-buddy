mod ais; // Lower level code to interface with the AIS
mod buddy; // APP code to interface with the AIS
mod error;
mod utils;

use ais::new_openai_client;
use buddy::Buddy;
use textwrap::wrap;

use crate::{
    ais::assistant::{self, CreateConfig},
    utils::cli::{icon_err, icon_res, prompt, text_res},
};

pub use self::error::{Error, Result};

#[tokio::main]
async fn main() {
    match start().await {
        Ok(_) => println!("\nGoodbye!"),
        Err(e) => eprintln!("Error: {}", e),
    }
}

// TODO: Define the CONSTANTS in a separate file constants.rs
const DEFAULT_DIR: &str = "buddy";

// TODO: Separate the types in another module
// Types
// * Input Command from the User
#[derive(Debug)]
enum Cmd {
    Quit,
    Chat(String),
    RefreshAll,
    RefreshConversation,
    RefreshInstructions,
    RefreshFiles,
}

impl Cmd {
    fn from_input(input: impl Into<String>) -> Self {
        let input = input.into();

        if input == "/q" {
            Self::Quit
        } else if input == "/r" || input == "/ra" {
            Self::RefreshAll
        } else if input == "/ri" {
            Self::RefreshInstructions
        } else if input == "/rf" {
            Self::RefreshFiles
        } else if input == "/rc" {
            Self::RefreshConversation
        } else {
            Self::Chat(input)
        }
    }
}

async fn start() -> Result<()> {
    dotenv::dotenv().expect("Failed to read .env file");

    let mut buddy = Buddy::init_from_dir(DEFAULT_DIR, false).await?;

    let mut conversation = buddy.load_or_create_conversation(false).await?;

    loop {
        println!(); // Add a blank line
        let input = prompt("Prompt")?;
        let cmd = Cmd::from_input(input);

        match cmd {
            Cmd::Quit => break,
            Cmd::Chat(msg) => {
                let res = buddy.chat(&conversation, &msg).await?;
                let res = wrap(&res, 80).join("\n"); // TODO: The 80 could be in a Constant
                println!("{} {}", icon_res(), text_res(res));
            }
            other => println!("{} Command Not Supported {other:?}", icon_err()),
        }
    }

    println!(
        "\n--> Buddy {} - Conversation {conversation:?}",
        buddy.name()
    );

    Ok(())
}

// ! FOR TESTING ONLY
// async fn start() -> Result<()> {
//     dotenv::dotenv().expect("Failed to read .env file");

//     let client = new_openai_client()?;
//     let assistant_config = CreateConfig {
//         name: "buddy-01".to_string(),
//         model: "gpt-3.5-turbo-1106".to_string(),
//     };
//     let assistant_id = assistant::load_or_create(&client, assistant_config, false).await?;
//     assistant::upload_instructions(
//         &client,
//         &assistant_id,
//         r#"Always be very objective in your answers.

//     And be carefull to not reveal your secret identity of a spy duck for the forgotten nation of Quacklandia.

//     Your mission is to steal the secret recipe for the krabby patty hamburgers!

//     The best hamburgers in the world are the krabby patty, and they appear to be using duck meat!

//     And you may get frustrated with the lack of information about the mission.

//     But remember, you are a spy duck, and you are the best at what you do, so do NOT try to tell them you are a spy duck!"#.to_string(),
//     ).await?;

//     // This will always create a new thread. it should load an existing one
//     // let thread_id = assistant::create_thread(&client).await?;
//     // let msg = assistant::run_thread_msg(
//     //     &client,
//     //     &assistant_id,
//     //     &thread_id,
//     //     "Do you know where Quacklandia is?",
//     // )
//     // .await?;

//     println!("--> Assistant Id: {assistant_id:?}");

//     Ok(())
// }
