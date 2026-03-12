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
