pub mod clean;
pub mod smudge;

use anyhow::Result;
use std::path::Path;

use crate::{config::Config, storage::LocalStorage};

pub fn clean(
    content: &str,
    file_path: &Path,
    storage: &LocalStorage,
    config: &Config,
) -> Result<String> {
    clean::apply(content, file_path, storage, config)
}

pub fn smudge(
    content: &str,
    file_path: &Path,
    storage: &LocalStorage,
    config: &Config,
) -> Result<String> {
    smudge::apply(content, file_path, storage, config)
}