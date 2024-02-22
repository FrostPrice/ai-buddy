use std::{
    collections::{HashMap, HashSet},
    path::Path,
    time::Duration,
};

use async_openai::types::{
    AssistantObject, AssistantToolsRetrieval, CreateAssistantFileRequest, CreateAssistantRequest,
    CreateFileRequest, CreateRunRequest, CreateThreadRequest, ModifyAssistantRequest, RunStatus,
    ThreadObject,
};
use console::Term;
use derive_more::{Deref, Display, From};
use serde::{de::IntoDeserializer, Deserialize, Serialize};
use tokio::time::sleep;

use crate::{
    ais::msg::{get_text_content, user_msg},
    utils::{
        cli::{icon_check, icon_deleted_ok, icon_err, icon_uploaded, icon_uploading},
        files::XFile,
    },
    Result,
};

use super::OpenAIClient;

// TODO: Define the CONSTANTS in a separate file constants.rs
// Constants
const DEFAULT_QUERY: &[(&str, &str)] = &[("limit", "100")];
const POLLING_DURATION_MS: u64 = 500;

pub struct CreateConfig {
    pub name: String,
    pub model: String,
}

// TODO: Implement Arc<String> to improve performance. Will be necessary to manually implement the From trait.
#[derive(Debug, Display, From, Deref)]
pub struct AssistantId(String);

#[derive(Debug, Display, From, Deref, Serialize, Deserialize)]
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
        println!("{} Assistant {} Deleted", icon_deleted_ok(), config.name);
    }

    // Load if exists
    if let Some(assistant_id) = assistant_id {
        println!("{} Assistant {} Loaded", icon_check(), config.name);
        Ok(assistant_id)
    } else {
        // Create if needed
        let assistant_name = config.name.clone();
        let assistant_id = create(openai_client, config).await?;
        println!("{} Assistant {} Created", icon_check(), assistant_name);
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

pub async fn get_thread(
    openai_client: &OpenAIClient,
    thread_id: &ThreadId,
) -> Result<ThreadObject> {
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

// Files
// * Return the File Id by File Name Hashmap
async fn get_files_hashmap(
    openai_client: &OpenAIClient,
    assistant_id: &AssistantId,
) -> Result<HashMap<String, FileId>> {
    // Get all Assistant Files (Files do NOT have .name)
    let openai_assistants = openai_client.assistants();
    let openai_assistant_files = openai_assistants.files(assistant_id);
    let assistant_files = openai_assistant_files.list(DEFAULT_QUERY).await?.data;
    let assistant_file_ids: HashSet<String> = assistant_files
        .into_iter()
        .map(|file_obj| file_obj.id)
        .collect();

    // Get all Files for Organization (Those files have .filename)
    let openai_files = openai_client.files();
    // let organization_files = openai_files.list([("purpose", "assistants")]).await?.data; // For async-openai version 0.18
    let organization_files = openai_files.list().await?.data;

    // Build or file_name::file_id Hashmap
    let file_id_by_name: HashMap<String, FileId> = organization_files
        .into_iter()
        .filter(|org_file| assistant_file_ids.contains(&org_file.id))
        .map(|org_file| (org_file.filename, org_file.id.into()))
        .collect();

    Ok(file_id_by_name)
}

// * Upload a file to an Assistant (Uploads first to the account, later then attaches to the Assistant)
// - `force` is `false`, will not upload file if already uploaded
// - `force` is `true`, it will delete the existing file (In the Account and Assistant), and then Upload
//
// Return `(FileId, has_been_uploaded)`
pub async fn upload_file_by_name(
    openai_client: &OpenAIClient,
    assistant_id: &AssistantId,
    file: &Path,
    force: bool,
) -> Result<(FileId, bool)> {
    let file_name = file.x_file_name();
    let mut file_id_by_name = get_files_hashmap(openai_client, assistant_id).await?;

    let file_id = file_id_by_name.remove(file_name);

    // If not force and file already create, return early
    if !force {
        if let Some(file_id) = file_id {
            return Ok((file_id, false));
        }
    }

    // If old file_id exists, delete the file
    if let Some(file_id) = file_id {
        // Delete the Organization File
        let openai_files = openai_client.files();
        if let Err(err) = openai_files.delete(&file_id).await {
            println!(
                "{} Cannot Delete File '{}'\n\tError: {}",
                icon_err(),
                file.x_file_name(),
                err
            );
        }

        // Delete the Assistant File Association
        let openai_assistant = openai_client.assistants();
        let openai_assistants_files = openai_assistant.files(assistant_id);
        if let Err(err) = openai_assistants_files.delete(&file_id).await {
            println!(
                "{} Cannot Remove Assistant File '{}'\n\tError: {}",
                icon_err(),
                file.x_file_name(),
                err
            );
        };
    }

    // Upload and Attach the File
    let term = Term::stdout();

    // Print Uploading
    term.write_line(&format!(
        "{} Uploading File '{}'",
        icon_uploading(),
        file.x_file_name()
    ))?;

    // Upload File
    let openai_files = openai_client.files();
    let openai_file = openai_files
        .create(CreateFileRequest {
            file: file.into(),
            purpose: "assistants".into(),
        })
        .await?;

    // Update Print
    term.clear_last_lines(1)?;
    term.write_line(&format!(
        "{} Uploaded File '{}'",
        icon_uploaded(),
        file.x_file_name()
    ))?;

    // Attach File to Assistant
    let openai_assistants = openai_client.assistants();
    let openai_assistant_files = openai_assistants.files(assistant_id);
    let assistant_file_obj = openai_assistant_files
        .create(CreateAssistantFileRequest {
            file_id: openai_file.id.clone(),
        })
        .await?;

    // Assert Warning
    if openai_file.id != assistant_file_obj.id {
        println!(
            "Critical Error: File Id do not match {} {}",
            openai_file.id, assistant_file_obj.id
        )
    }

    Ok((assistant_file_obj.id.into(), true))
}
