pub mod project;

pub use project::Config;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigData {
    pub version: String,
    pub encryption: EncryptionConfig,
    pub git: GitConfig,
    pub decorations: DecorationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub enabled: bool,
    pub algorithm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    pub auto_sync: bool,
    pub pre_commit_check: bool,
    pub diff_driver: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecorationConfig {
    pub inline_pattern: String,
    pub block_start: String,
    pub block_end: String,
    pub partial_pattern: String,
}

impl Default for ConfigData {
    fn default() -> Self {
        Self {
            version: "0.1.0".to_string(),
            encryption: EncryptionConfig {
                enabled: false,
                algorithm: "aes-256-gcm".to_string(),
            },
            git: GitConfig {
                auto_sync: true,
                pre_commit_check: true,
                diff_driver: false,
            },
            decorations: DecorationConfig {
                inline_pattern: "@whiteout:".to_string(),
                block_start: "@whiteout-start".to_string(),
                block_end: "@whiteout-end".to_string(),
                partial_pattern: r"\[\[.*\|\|.*\]\]".to_string(),
            },
        }
    }
}