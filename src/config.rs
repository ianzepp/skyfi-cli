use crate::error::CliError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub api: ApiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("skyfi")
            .join("config.toml")
    }

    pub fn redacted(&self) -> Self {
        let mut config = self.clone();
        if config.api.api_key.is_some() {
            config.api.api_key = Some("[redacted]".to_string());
        }
        config
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
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ApiConfig, Config};

    #[test]
    fn redacted_masks_existing_api_key() {
        let config = Config {
            api: ApiConfig {
                base_url: "https://example.com/platform-api".to_string(),
                api_key: Some("secret".to_string()),
            },
        };

        let redacted = config.redacted();

        assert_eq!(redacted.api.base_url, config.api.base_url);
        assert_eq!(redacted.api.api_key.as_deref(), Some("[redacted]"));
        assert_eq!(config.api.api_key.as_deref(), Some("secret"));
    }

    #[test]
    fn redacted_leaves_missing_api_key_unset() {
        let config = Config::default();

        let redacted = config.redacted();

        assert!(redacted.api.api_key.is_none());
    }
}
