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
mod tests {
    use super::validate_base_url;

    #[test]
    fn validate_base_url_accepts_absolute_urls() {
        assert!(validate_base_url("https://app.skyfi.com/platform-api").is_ok());
    }

    #[test]
    fn validate_base_url_rejects_invalid_urls() {
        let error = validate_base_url("not a url").expect_err("invalid URL should be rejected");
        assert!(error.to_string().contains("invalid base URL"));
    }
}
