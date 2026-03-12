use super::api_error_message;

#[test]
fn api_error_message_extracts_detail_field() {
    let body = r#"{"detail":"forbidden"}"#;

    assert_eq!(api_error_message(body).as_deref(), Some("forbidden"));
}

#[test]
fn api_error_message_returns_none_for_non_json_bodies() {
    assert_eq!(api_error_message("gateway timeout"), None);
}
