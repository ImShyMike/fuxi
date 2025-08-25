use std::{collections::HashMap, env, fs, path::PathBuf};

use chrono::{DateTime, Utc};
use config::{Config, File, FileFormat};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FuxiConfig {
    pub platform: Option<String>,
    pub selected_profile: Option<String>,
    pub profiles: Option<HashMap<String, Vec<String>>>,
    pub last_backup_id: Option<String>,
    pub backup_repo_path: Option<String>,
    pub github_repo: Option<String>,
    pub git_branch: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BackupMetadata {
    id: String,
    timestamp: DateTime<Utc>,
    paths: Vec<String>,
    commit_hash: Option<String>,
    description: Option<String>,
}

impl Default for FuxiConfig {
    fn default() -> Self {
        Self {
            platform: env::consts::OS.to_string().into(),
            selected_profile: None,
            profiles: None,
            last_backup_id: None,
            backup_repo_path: None,
            github_repo: None,
            git_branch: "main".to_string(),
        }
    }
}

pub fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config_dir = dirs::config_dir().ok_or("Could not determine config directory")?;
    let app_config_dir = config_dir.join("fuxi");

    // Create the config directory if it doesn't exist
    std::fs::create_dir_all(&app_config_dir)?;

    Ok(app_config_dir.join("config.toml"))
}

pub fn load_config() -> Result<FuxiConfig, Box<dyn std::error::Error>> {
    let config_path = get_config_path()?;

    let mut builder = Config::builder();

    // Add config file if it exists
    if config_path.exists() {
        builder = builder.add_source(
            File::from(config_path.clone())
                .format(FileFormat::Toml)
                .required(false),
        );
    }

    let config = builder.build()?;

    // Try to deserialize into our struct, fall back to default if it fails
    match config.try_deserialize::<FuxiConfig>() {
        Ok(fuxi_config) => Ok(fuxi_config),
        Err(_) => {
            // If deserialization fails, return default
            Ok(FuxiConfig::default())
        }
    }
}

pub fn save_config(config: &FuxiConfig) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path()?;
    let config_str = toml::to_string_pretty(config)?;
    fs::write(config_path, config_str)?;
    Ok(())
}
