use serde::Deserialize;

use crate::ais::assistant;

#[derive(Debug, Deserialize)]
pub(super) struct Config {
    pub name: String,
    model: String,
    pub instructions_file: String,
    file_bundles: Vec<FileBundle>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FileBundle {
    bundle_name: String,
    src_dir: String,
    dst_ext: String,
    src_globs: Vec<String>,
}

// Froms
impl From<&Config> for assistant::CreateConfig {
    fn from(config: &Config) -> Self {
        Self {
            name: config.name.clone(),
            model: config.model.clone(),
        }
    }
}
