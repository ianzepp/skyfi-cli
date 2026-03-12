use super::to_pretty_json;
use serde_json::json;

#[test]
fn to_pretty_json_preserves_object_fields() {
    let rendered = to_pretty_json(&json!({
        "message": "ok",
        "count": 2
    }))
    .expect("json should serialize");

    assert!(rendered.contains("\"message\": \"ok\""));
    assert!(rendered.contains("\"count\": 2"));
}
