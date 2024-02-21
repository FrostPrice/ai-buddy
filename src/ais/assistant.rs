use std::time::Duration;

use async_openai::types::{
    AssistantObject, AssistantToolsRetrieval, CreateAssistantRequest, CreateRunRequest,
    CreateThreadRequest, ModifyAssistantRequest, RunStatus, ThreadObject,
};
use console::Term;
use derive_more::{Deref, Display, From};
use tokio::time::sleep;

use crate::{
    ais::msg::{get_text_content, user_msg},
    Result,
};

use super::OpenAIClient;

// TODO: Define the CONSTANTS in a separate file constants.rs
// Constants
const DEFAULT_QUERY: &[(&str, &str)] = &[("limit", "100")];
const POLLING_DURATION_MS: u64 = 500;

// TODO: This info could be stored in the .env file.
pub struct CreateConfig {
    pub name: String,
    pub model: String,
}

// TODO: Implement Arc<String> to improve performance. Will be necessary to manually implement the From trait.
#[derive(Debug, Display, From, Deref)]
pub struct AssistantId(String);

#[derive(Debug, Display, From, Deref)]
pub struct ThreadId(String);

#[derive(Debug, Display, From, Deref)]
pub struct FileId(String);

// CRUD
async fn create(openai_client: &OpenAIClient, config: CreateConfig) -> Result<AssistantId> {
    let openai_assistants = openai_client.assistants();

    let assistant_object = openai_assistants
        .create(CreateAssistantRequest {
            name: Some(config.name),
            model: config.model,
            tools: Some(vec![AssistantToolsRetrieval::default().into()]),
            ..Default::default()
        })
        .await?;

    Ok(assistant_object.id.into())
}

pub async fn load_or_create(
    openai_client: &OpenAIClient,
    config: CreateConfig,
    recreate: bool,
) -> Result<AssistantId> {
    let assistant_object = find_by_name(openai_client, &config.name).await?;
    let mut assistant_id = assistant_object.map(|obj| AssistantId::from(obj.id));

    // Delete Assistant if recreate true and assistant_id exists
    if let (true, Some(assistant_id_ref)) = (recreate, assistant_id.as_ref()) {
        delete(openai_client, assistant_id_ref).await?;
        assistant_id.take();
        println!("Assistant {} Deleted", config.name);
    }

    // Load if exists
    if let Some(assistant_id) = assistant_id {
        println!("Assistant {} Loaded", config.name);
        Ok(assistant_id)
    } else {
        // Create if needed
        let assistant_name = config.name.clone();
        let assistant_id = create(openai_client, config).await?;
        println!("Assistant {} Created", assistant_name);
        Ok(assistant_id)
    }
}

async fn find_by_name(openai_client: &OpenAIClient, name: &str) -> Result<Option<AssistantObject>> {
    let openai_assistants = openai_client.assistants();

    // TODO: There could be a pagination to this query
    let assistants = openai_assistants.list(DEFAULT_QUERY).await?.data;

    let assistant_object = assistants.into_iter().find(|asst| {
        asst.name
            .as_ref()
            .map(|asst_name| asst_name == name)
            .unwrap_or(false)
    });

    Ok(assistant_object)
}

pub async fn upload_instructions(
    openai_client: &OpenAIClient,
    assistant_id: &AssistantId,
    instruction_content: String,
) -> Result<()> {
    let openai_assistants = openai_client.assistants();
    let modify = ModifyAssistantRequest {
        instructions: Some(instruction_content),
        ..Default::default()
    };
    openai_assistants.update(assistant_id, modify).await?;

    Ok(())
}

async fn delete(openai_client: &OpenAIClient, assistant_id: &AssistantId) -> Result<()> {
    let openai_assistants = openai_client.assistants();

    // TODO: Delete files
    // Delete Assistant
    openai_assistants.delete(assistant_id).await?;

    Ok(())
}

// Thread
pub async fn create_thread(openai_client: &OpenAIClient) -> Result<ThreadId> {
    let openai_threads = openai_client.threads();

    let res = openai_threads
        .create(CreateThreadRequest {
            ..Default::default()
        })
        .await?;

    Ok(res.id.into())
}

async fn get_thread(openai_client: &OpenAIClient, thread_id: &ThreadId) -> Result<ThreadObject> {
    let openai_threads = openai_client.threads();

    let thread_object = openai_threads.retrieve(thread_id).await?;

    Ok(thread_object)
}

pub async fn run_thread_msg(
    openai_client: &OpenAIClient,
    assistant_id: &AssistantId,
    thread_id: &ThreadId,
    msg: &str,
) -> Result<String> {
    let msg = user_msg(msg);

    // Attach message to thread
    let _message_obj = openai_client
        .threads()
        .messages(thread_id)
        .create(msg)
        .await?;

    // Create a run for the thread
    let run_req = CreateRunRequest {
        assistant_id: assistant_id.to_string(),
        ..Default::default()
    };
    let run = openai_client
        .threads()
        .runs(thread_id)
        .create(run_req)
        .await?;
    let run_id = run.id;

    // Loop to get the result
    let term = Term::stdout();
    loop {
        term.write_str(">")?;
        let run = openai_client
            .threads()
            .runs(thread_id)
            .retrieve(&run_id)
            .await?;
        term.write_str("< ")?;
        match run.status {
            RunStatus::Completed => {
                term.write_str("\n")?;
                return get_first_thread_msg_content(openai_client, thread_id).await;
            }
            RunStatus::Queued | RunStatus::InProgress => (),
            other => {
                term.write_str("\n")?;
                return Err(format!("Error while Run: {:?}", other).into());
            }
        }

        sleep(Duration::from_millis(POLLING_DURATION_MS)).await;
    }
}

async fn get_first_thread_msg_content(
    openai_client: &OpenAIClient,
    thread_id: &ThreadId,
) -> Result<String> {
    static QUERY: [(&str, &str); 1] = [("limit", "1")];

    let messages = openai_client
        .threads()
        .messages(thread_id)
        .list(&QUERY)
        .await?;

    let msg = messages
        .data
        .into_iter()
        .next()
        .ok_or_else(|| "No message found".to_string())?;

    let text = get_text_content(msg)?;

    Ok(text)
}
