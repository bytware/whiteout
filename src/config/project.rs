use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use super::ConfigData;

#[derive(Debug, Clone)]
pub struct Config {
    pub data: ConfigData,
    pub path: PathBuf,
}

impl Config {
    pub fn load_or_default(project_root: impl AsRef<Path>) -> Result<Self> {
        let config_path = project_root.as_ref().join(".whiteout").join("config.toml");
        
        let data = if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .context("Failed to read config file")?;
            toml::from_str(&content).context("Failed to parse config file")?
        } else {
            ConfigData::default()
        };
        
        Ok(Self {
            data,
            path: config_path,
        })
    }

    pub fn init(project_root: impl AsRef<Path>) -> Result<()> {
        let whiteout_dir = project_root.as_ref().join(".whiteout");
        fs::create_dir_all(&whiteout_dir).context("Failed to create .whiteout directory")?;
        
        let config_path = whiteout_dir.join("config.toml");
        if !config_path.exists() {
            let initial_config = ConfigData::default();
            let content = toml::to_string_pretty(&initial_config)
                .context("Failed to serialize initial config")?;
            fs::write(&config_path, content).context("Failed to write initial config")?;
        }
        
        Self::setup_git_config(project_root.as_ref())?;
        
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let content = toml::to_string_pretty(&self.data)
            .context("Failed to serialize config")?;
        
        fs::create_dir_all(self.path.parent().unwrap())
            .context("Failed to create config directory")?;
        
        fs::write(&self.path, content)
            .context("Failed to write config file")?;
        
        Ok(())
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "encryption.enabled" => {
                self.data.encryption.enabled = value.parse()
                    .context("Invalid boolean value")?;
            }
            "git.auto_sync" => {
                self.data.git.auto_sync = value.parse()
                    .context("Invalid boolean value")?;
            }
            "git.pre_commit_check" => {
                self.data.git.pre_commit_check = value.parse()
                    .context("Invalid boolean value")?;
            }
            _ => anyhow::bail!("Unknown config key: {}", key),
        }
        
        self.save()?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<String> {
        let value = match key {
            "encryption.enabled" => self.data.encryption.enabled.to_string(),
            "git.auto_sync" => self.data.git.auto_sync.to_string(),
            "git.pre_commit_check" => self.data.git.pre_commit_check.to_string(),
            _ => anyhow::bail!("Unknown config key: {}", key),
        };
        
        Ok(value)
    }

    fn setup_git_config(project_root: &Path) -> Result<()> {
        let gitattributes_path = project_root.join(".gitattributes");
        let mut content = if gitattributes_path.exists() {
            fs::read_to_string(&gitattributes_path)?
        } else {
            String::new()
        };
        
        if !content.contains("filter=whiteout") {
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str("* filter=whiteout\n");
            fs::write(&gitattributes_path, content)?;
        }
        
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data: ConfigData::default(),
            path: PathBuf::from(".whiteout/config.toml"),
        }
    }
}