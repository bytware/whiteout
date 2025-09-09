pub mod config;
pub mod parser;
pub mod storage;
pub mod transform;

use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Whiteout {
    config: config::Config,
    storage: storage::LocalStorage,
}

impl Whiteout {
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        let project_root = project_root.as_ref();
        let config = config::Config::load_or_default(project_root)?;
        let storage = storage::LocalStorage::new(project_root)?;
        
        Ok(Self { config, storage })
    }

    pub fn init(project_root: impl AsRef<Path>) -> Result<Self> {
        let project_root = project_root.as_ref();
        config::Config::init(project_root)?;
        storage::LocalStorage::init(project_root)?;
        
        Self::new(project_root)
    }

    pub fn clean(&self, content: &str, file_path: &Path) -> Result<String> {
        transform::clean(content, file_path, &self.storage, &self.config)
    }

    pub fn smudge(&self, content: &str, file_path: &Path) -> Result<String> {
        transform::smudge(content, file_path, &self.storage, &self.config)
    }
}