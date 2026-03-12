use crate::error::CliError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub api: ApiConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiConfig {
    #[serde(default = "default_base_url")]
    pub base_url: String,
    pub api_key: Option<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            api_key: None,
        }
    }
}

fn default_base_url() -> String {
    "https://app.skyfi.com/platform-api".to_string()
}

impl Config {
    pub fn path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("skyfi")
            .join("config.toml")
    }

    pub fn load(path: &Path) -> Result<Self, CliError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)?;
        toml::from_str(&content).map_err(|e| CliError::Config(format!("parse error: {e}")))
    }

    pub fn save(&self, path: &Path) -> Result<(), CliError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content =
            toml::to_string_pretty(self).map_err(|e| CliError::Config(format!("serialize: {e}")))?;
        fs::write(path, content)?;
        Ok(())
    }
}
