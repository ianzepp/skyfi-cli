//! Configuration file loading, saving, and validation.
//!
//! Configuration lives at `~/.config/skyfi/config.toml` (resolved via the
//! `dirs` crate for cross-platform correctness). The file and its parent
//! directory are created automatically on first write.
//!
//! The config intentionally stores only two pieces of state: the API base URL
//! and the API key. Everything else is passed as CLI flags at runtime. This
//! keeps the on-disk format stable and the config easy to inspect.
//!
//! TRADE-OFFS:
//! - The API key is stored in plaintext. This is a deliberate choice for
//!   usability — the alternative (OS keychain integration) adds per-platform
//!   complexity for a tool whose primary audience is developers and operators
//!   who understand credential file hygiene. The `SKYFI_API_KEY` env var
//!   provides a secrets-manager-friendly alternative.
//! - `Config::redacted()` exists so `config show` never echoes the raw key
//!   back to the terminal or into log output.

use crate::error::CliError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Top-level configuration structure, serialized as TOML.
///
/// Fields are grouped under an `[api]` table to leave room for future
/// sections (e.g. `[output]`, `[cache]`) without breaking the existing format.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub api: ApiConfig,
}

/// API connection settings stored under the `[api]` TOML table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Base URL for the SkyFi Platform API. Defaults to the production endpoint.
    #[serde(default = "default_base_url")]
    pub base_url: String,

    /// SkyFi API key. `None` if not yet configured. The `Client` also checks
    /// `SKYFI_API_KEY` at runtime, which takes precedence over this field.
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

/// Returns the production API base URL, used as the `serde` default for `base_url`.
fn default_base_url() -> String {
    "https://app.skyfi.com/platform-api".to_string()
}

impl Config {
    /// Returns the platform-appropriate default config file path.
    ///
    /// Uses `dirs::config_dir()` for cross-platform support (`~/.config` on Linux/macOS,
    /// `%APPDATA%` on Windows). Falls back to `.config` relative to the working directory
    /// if the system config directory cannot be determined.
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("skyfi")
            .join("config.toml")
    }

    /// Returns a copy of this config with the API key replaced by `"[redacted]"`.
    ///
    /// WHY: used by `config show` so the raw API key is never printed to the
    /// terminal or captured in shell history or logs.
    pub fn redacted(&self) -> Self {
        let mut config = self.clone();
        if config.api.api_key.is_some() {
            config.api.api_key = Some("[redacted]".to_string());
        }
        config
    }

    /// Load configuration from `path`, returning `Config::default()` if the file does not exist.
    ///
    /// A missing config file is treated as an empty config (not an error) so the CLI
    /// works out of the box with only the `SKYFI_API_KEY` environment variable set.
    pub fn load(path: &Path) -> Result<Self, CliError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)?;
        toml::from_str(&content).map_err(|e| CliError::Config(format!("parse error: {e}")))
    }

    /// Serialize and write this config to `path`, creating parent directories as needed.
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
#[path = "config_test.rs"]
mod tests;
