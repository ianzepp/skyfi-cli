use super::feasibility_status;
use serde_json::json;

#[test]
fn feasibility_status_reads_status_field() {
    let value = json!({ "status": "COMPLETE" });
    assert_eq!(feasibility_status(&value), Some("COMPLETE"));
}

#[test]
fn feasibility_status_returns_none_when_missing() {
    let value = json!({ "id": "abc" });
    assert_eq!(feasibility_status(&value), None);
}
