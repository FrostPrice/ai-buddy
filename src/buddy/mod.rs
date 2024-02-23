mod config;

use std::{
    fs,
    path::{Path, PathBuf},
};

use derive_more::{Deref, From};
use serde::{Deserialize, Serialize};

use crate::{
    ais::{
        assistant::{self, AssistantId, ThreadId},
        new_openai_client, OpenAIClient,
    },
    utils::{
        cli::icon_check,
        files::{
            bundle_to_file, ensure_dir, list_files, load_from_json, load_from_toml, read_to_string,
            save_to_json,
        },
    },
    Result,
};

use self::config::Config;

// TODO: Define the CONSTANTS in a separate file constants.rs
const BUDDY_TOML: &str = "buddy.toml";

// TODO: Implement Arc<T> to improve performance. Mayvbe will be necessary to manually implement the From trait.
#[derive(Debug)]
pub struct Buddy {
    dir: PathBuf,
    openai_client: OpenAIClient,
    assistant_id: AssistantId,
    config: Config,
}

#[derive(Debug, From, Deref, Serialize, Deserialize)]
pub struct Conversation {
    thread_id: ThreadId,
}

// * Public Functions
impl Buddy {
    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub async fn init_from_dir(dir: impl AsRef<Path>, recreate_assistant: bool) -> Result<Self> {
        let dir = dir.as_ref();

        // Load from the Directory
        let config: Config = load_from_toml(dir.join(BUDDY_TOML))?;

        // Get or Create the OpenAI Assistant
        let openai_client = new_openai_client()?;
        let assistant_id =
            assistant::load_or_create(&openai_client, (&config).into(), recreate_assistant).await?;

        // Create Buddy
        let buddy = Buddy {
            dir: dir.to_path_buf(),
            openai_client,
            assistant_id,
            config,
        };

        // Upload the Instructions
        buddy.upload_instructions().await?;

        // Upload Files
        buddy.upload_files(false).await?;

        Ok(buddy)
    }

    pub async fn upload_instructions(&self) -> Result<bool> {
        let file = self.dir.join(&self.config.instructions_file);

        // If files exists, try to upload it. Else returns false
        if file.exists() {
            let instruction_content = read_to_string(&file)?;
            assistant::upload_instructions(
                &self.openai_client,
                &self.assistant_id,
                instruction_content,
            )
            .await?;
            println!("{} Instructions Uploaded", icon_check());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn upload_files(&self, recreate: bool) -> Result<u32> {
        let mut num_uploaded = 0;

        // Get the .buddy/files
        let data_files_dir = self.data_files_dir()?;

        // Clean the .buddy/files left over by Old Buddy
        let exclude_element = format!("*{}*", &self.assistant_id);
        for file in list_files(
            &data_files_dir,
            Some(&["*.rs", "*.md"]),
            Some(&[&exclude_element]),
        )? {
            // Delete the file
            let file_str = file.to_string_lossy();
            // Safeguard
            if !file_str.contains(".buddy") {
                return Err(format!("Error Should Not Delete: '{}'", file_str).into());
            }
            fs::remove_file(&file)?;
        }

        // Generate and Upload the .buddy/files bundle files
        for bundle in self.config.file_bundles.iter() {
            let src_dir = self.dir.join(&bundle.src_dir);

            if src_dir.is_dir() {
                let src_globs: Vec<&str> = bundle.src_globs.iter().map(AsRef::as_ref).collect();

                let files = list_files(&src_dir, Some(&src_globs), None)?;

                if !files.is_empty() {
                    // Compute the Bundle File Name
                    let bundle_file_name = format!(
                        "{}-{}-bundle-{}.{}",
                        self.name(),
                        bundle.bundle_name,
                        self.assistant_id,
                        bundle.dst_ext
                    );
                    let bundle_file = self.data_files_dir()?.join(bundle_file_name);

                    // If it doesn't exists, then force a reupload
                    let force_reupload = recreate || !bundle_file.exists();

                    // Rebundle no matter if it exists or not
                    bundle_to_file(files, &bundle_file)?;

                    // Upload
                    let (_, uploaded) = assistant::upload_file_by_name(
                        &self.openai_client,
                        &self.assistant_id,
                        &bundle_file,
                        force_reupload,
                    )
                    .await?;

                    if uploaded {
                        num_uploaded += 1;
                    }
                }
            }
        }

        Ok(num_uploaded)
    }

    pub async fn load_or_create_conversation(&self, recreate: bool) -> Result<Conversation> {
        let conversation_file = self.data_dir()?.join("conversation.json");

        if recreate && conversation_file.exists() {
            fs::remove_file(&conversation_file)?;
        }

        let conversation =
            if let Ok(conversation) = load_from_json::<Conversation>(&conversation_file) {
                assistant::get_thread(&self.openai_client, &conversation.thread_id)
                    .await
                    .map_err(|_| format!("Cannot find thread_id for {:?}", conversation))?;
                println!("{} Conversation Loaded", icon_check());
                conversation
            } else {
                let thread_id = assistant::create_thread(&self.openai_client).await?;
                println!("{} Conversation Created", icon_check());
                let conversation = thread_id.into();
                save_to_json(&conversation_file, &conversation)?;
                conversation
            };

        Ok(conversation)
    }

    pub async fn chat(&self, conversation: &Conversation, msg: &str) -> Result<String> {
        let res = assistant::run_thread_msg(
            &self.openai_client,
            &self.assistant_id,
            &conversation.thread_id,
            msg,
        )
        .await?;

        Ok(res)
    }
}

// * Private Functions
impl Buddy {
    fn data_dir(&self) -> Result<PathBuf> {
        let data_dir = self.dir.join(".buddy");
        ensure_dir(&data_dir)?;
        Ok(data_dir)
    }

    fn data_files_dir(&self) -> Result<PathBuf> {
        let dir = self.data_dir()?.join("files");
        ensure_dir(&dir)?;
        Ok(dir)
    }
}
