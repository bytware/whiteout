pub mod crypto;
pub mod local;

pub use local::LocalStorage;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageEntry {
    pub file_path: PathBuf,
    pub key: String,
    pub value: String,
    pub encrypted: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageData {
    pub entries: HashMap<String, StorageEntry>,
    pub version: String,
}