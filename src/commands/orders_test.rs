use super::{deserialize_passes, enum_query_value, prediction_date, select_pass};
use crate::types::{SortColumn, SortDirection};
use serde_json::json;

#[test]
fn enum_query_value_uses_serde_names_for_sort_columns() {
    let value = enum_query_value(&SortColumn::CreatedAt, "sort column")
        .expect("sort column should serialize");
    assert_eq!(value, "created_at");
}

#[test]
fn enum_query_value_uses_serde_names_for_sort_directions() {
    let value = enum_query_value(&SortDirection::Desc, "sort direction")
        .expect("sort direction should serialize");
    assert_eq!(value, "desc");
}

#[test]
fn prediction_date_normalizes_rfc3339_timestamps() {
    let date = prediction_date("2025-04-01T05:30:00Z").expect("timestamp should parse");
    assert_eq!(date, "2025-04-01");
}

#[test]
fn select_pass_returns_requested_provider_window_id() {
    let passes = deserialize_passes(vec![
        json!({
            "providerWindowId": "second",
            "provider": "PLANET",
            "passDate": "2025-04-03T00:00:00Z"
        }),
        json!({
            "providerWindowId": "first",
            "provider": "UMBRA",
            "passDate": "2025-04-01T00:00:00Z"
        }),
    ])
    .expect("passes should deserialize");

    let selected = select_pass(&passes, Some("second")).expect("pass should exist");
    assert_eq!(selected.provider_window_id, "second");
}

#[test]
fn select_pass_uses_earliest_pass_date_by_default() {
    let passes = deserialize_passes(vec![
        json!({
            "providerWindowId": "later",
            "passDate": "2025-04-03T00:00:00Z"
        }),
        json!({
            "providerWindowId": "earlier",
            "passDate": "2025-04-01T00:00:00Z"
        }),
    ])
    .expect("passes should deserialize");

    let selected = select_pass(&passes, None).expect("one pass should be selected");
    assert_eq!(selected.provider_window_id, "earlier");
}
