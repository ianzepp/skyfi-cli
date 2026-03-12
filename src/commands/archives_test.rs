use super::display_date_prefix;

#[test]
fn display_date_prefix_limits_output_to_ten_characters() {
    assert_eq!(display_date_prefix("2025-04-01T12:00:00Z"), "2025-04-01");
}

#[test]
fn display_date_prefix_is_safe_for_short_or_non_ascii_strings() {
    assert_eq!(display_date_prefix("éclair"), "éclair");
}
