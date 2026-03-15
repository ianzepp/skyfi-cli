use super::{artifact_from_paths, bbox_to_wkt, render_report, ResearchResult, ResearchToolCall};
use serde_json::json;
use std::path::Path;

#[test]
fn bbox_to_wkt_formats_polygon() {
    let polygon = bbox_to_wkt(37.7, 37.8, -122.4, -122.3);
    assert_eq!(
        polygon,
        "POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))"
    );
}

#[test]
fn render_report_includes_analysis_and_tool_trace() {
    let result = ResearchResult {
        markdown: "## Findings\n\nUseful imagery exists.".to_string(),
        response_id: "resp_123".to_string(),
        model: "gpt-4.1".to_string(),
        tool_calls: vec![ResearchToolCall {
            step: 1,
            name: "resolve_location".to_string(),
            arguments: json!({ "query": "Port of Sudan" }),
            output: json!({ "results": [{ "wkt_polygon": "POLYGON ((...))" }] }),
            is_error: false,
        }],
    };

    let report = render_report("Investigate the port", &result);
    assert!(report.contains("# SkyFi Research Brief"));
    assert!(report.contains("## Objective"));
    assert!(report.contains("Investigate the port"));
    assert!(report.contains("## Findings"));
    assert!(report.contains("### 1. `resolve_location`"));
    assert!(report.contains("\"query\": \"Port of Sudan\""));
}

#[test]
fn artifact_captures_paths_and_metadata() {
    let result = ResearchResult {
        markdown: "Findings".to_string(),
        response_id: "resp_123".to_string(),
        model: "gpt-4.1".to_string(),
        tool_calls: Vec::new(),
    };

    let artifact = artifact_from_paths(
        "Investigate the port",
        &result,
        Path::new("brief.md"),
        Some(Path::new("trace.json")),
    );

    assert_eq!(artifact.prompt, "Investigate the port");
    assert_eq!(artifact.markdown_path, "brief.md");
    assert_eq!(artifact.trace_path.as_deref(), Some("trace.json"));
    assert_eq!(artifact.response_id, "resp_123");
}
