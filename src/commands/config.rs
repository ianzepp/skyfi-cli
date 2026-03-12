//! Config file management commands.
//!
//! Config commands are special: they are handled before the HTTP `Client` is
//! constructed (see `main::run`). This means they work without a valid API key,
//! which is intentional — you need to be able to run `config set-key` to
//! bootstrap authentication.
//!
//! # Commands
//!
//! - `config show` — print the current config, with the API key redacted.
//! - `config set-key <KEY>` — save an API key to the config file.
//! - `config set-url <URL>` — override the API base URL (validated before saving).

use crate::cli::ConfigAction;
use crate::config::Config;
use crate::error::CliError;
use std::path::Path;

/// Validate that `url` is a well-formed URL before saving it to the config.
///
/// WHY: An invalid base URL would cause every subsequent API command to fail
/// with a confusing reqwest error. Validating at write time surfaces the problem
/// at the point of misconfiguration.
fn validate_base_url(url: &str) -> Result<(), CliError> {
    reqwest::Url::parse(url)
        .map(|_| ())
        .map_err(|error| CliError::Config(format!("invalid base URL: {error}")))
}

pub fn run(action: ConfigAction, config: &mut Config, config_path: &Path) -> Result<(), CliError> {
    match action {
        ConfigAction::Show => {
            let content = toml::to_string_pretty(&config.redacted())?;
            println!("{content}");
        }
        ConfigAction::SetKey { key } => {
            config.api.api_key = Some(key);
            config.save(config_path)?;
            eprintln!("API key saved to {}", config_path.display());
        }
        ConfigAction::SetUrl { url } => {
            validate_base_url(&url)?;
            config.api.base_url = url;
            config.save(config_path)?;
            eprintln!("Base URL saved to {}", config_path.display());
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "config_test.rs"]
mod tests;
