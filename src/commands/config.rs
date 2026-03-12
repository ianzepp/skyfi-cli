use crate::cli::ConfigAction;
use crate::config::Config;
use crate::error::CliError;
use std::path::Path;

pub fn run(action: ConfigAction, config: &mut Config, config_path: &Path) -> Result<(), CliError> {
    match action {
        ConfigAction::Show => {
            let content = toml::to_string_pretty(config)
                .map_err(|e| CliError::Config(format!("serialize: {e}")))?;
            println!("{content}");
        }
        ConfigAction::SetKey { key } => {
            config.api.api_key = Some(key);
            config.save(config_path)?;
            eprintln!("API key saved to {}", config_path.display());
        }
        ConfigAction::SetUrl { url } => {
            config.api.base_url = url;
            config.save(config_path)?;
            eprintln!("Base URL saved to {}", config_path.display());
        }
    }
    Ok(())
}
