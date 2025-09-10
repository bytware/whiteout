use anyhow::{Context, Result, bail};
use colored::Colorize;
use crate::cli::ConfigAction;

pub fn handle(action: ConfigAction) -> Result<()> {
    let config_path = std::path::Path::new(".whiteout/config.toml");
    
    match action {
        ConfigAction::Set { key, value } => {
            println!("{} Setting {} = {}", 
                "→".bright_green(), 
                key.bright_cyan(), 
                value.bright_yellow());
            
            // Load existing config
            let mut config = if config_path.exists() {
                let content = std::fs::read_to_string(config_path)
                    .context("Failed to read config file")?;
                toml::from_str(&content)
                    .context("Failed to parse config file")?
            } else {
                toml::Value::Table(toml::map::Map::new())
            };
            
            // Set the value
            if let Some(table) = config.as_table_mut() {
                table.insert(key.clone(), toml::Value::String(value));
                
                // Write back
                std::fs::create_dir_all(config_path.parent().unwrap())?;
                std::fs::write(config_path, toml::to_string_pretty(&config)?)
                    .context("Failed to write config file")?;
                
                println!("{} Configuration updated", "✓".bright_green());
            } else {
                bail!("Invalid config structure");
            }
        }
        
        ConfigAction::Get { key } => {
            if !config_path.exists() {
                println!("{} No configuration file found", "⚠".bright_yellow());
                return Ok(());
            }
            
            let content = std::fs::read_to_string(config_path)
                .context("Failed to read config file")?;
            let config: toml::Value = toml::from_str(&content)
                .context("Failed to parse config file")?;
            
            if let Some(table) = config.as_table() {
                if let Some(value) = table.get(&key) {
                    println!("{} = {}", key.bright_cyan(), value);
                } else {
                    println!("{} Key '{}' not found", "⚠".bright_yellow(), key);
                }
            }
        }
        
        ConfigAction::List => {
            if !config_path.exists() {
                println!("{} No configuration file found", "⚠".bright_yellow());
                return Ok(());
            }
            
            let content = std::fs::read_to_string(config_path)
                .context("Failed to read config file")?;
            let config: toml::Value = toml::from_str(&content)
                .context("Failed to parse config file")?;
            
            println!("{}", "Current Configuration:".bright_blue().bold());
            
            if let Some(table) = config.as_table() {
                if table.is_empty() {
                    println!("  {} No configuration values set", "ℹ".bright_blue());
                } else {
                    for (key, value) in table {
                        println!("  {} = {}", 
                            key.bright_cyan(), 
                            value.to_string().bright_yellow());
                    }
                }
            }
        }
    }
    
    Ok(())
}