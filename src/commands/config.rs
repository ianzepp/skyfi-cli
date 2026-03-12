use crate::cli::ConfigAction;
use crate::config::Config;
use crate::error::CliError;
use std::path::Path;

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
