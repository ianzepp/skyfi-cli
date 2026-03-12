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
