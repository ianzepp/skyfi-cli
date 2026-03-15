use crate::client::Client;
use crate::error::CliError;
use crate::types::{
    FeasibilityRequest, GetArchivesRequest, PassPredictionRequest, PricingRequest, ProductType,
};
use chrono::Utc;
use futures_util::StreamExt;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const DEFAULT_MODEL: &str = "gpt-4.1";
const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";

#[derive(Debug, Serialize)]
pub struct ResearchRunArtifact {
    pub prompt: String,
    pub model: String,
    pub generated_at: String,
    pub markdown_path: String,
    pub trace_path: Option<String>,
    pub response_id: String,
    pub tool_calls: Vec<ResearchToolCall>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResearchToolCall {
    pub step: usize,
    pub name: String,
    pub arguments: Value,
    pub output: Value,
    pub is_error: bool,
}

#[derive(Debug)]
pub struct ResearchResult {
    pub markdown: String,
    pub response_id: String,
    pub model: String,
    pub tool_calls: Vec<ResearchToolCall>,
}

pub enum ResearchProgress {
    ModelRoundStarted {
        step: usize,
    },
    ModelTextDelta(String),
    ToolCallRequested {
        step: usize,
        name: String,
    },
    ToolResult {
        step: usize,
        name: String,
        is_error: bool,
    },
}

#[derive(Debug, Deserialize)]
struct ResponsesApiResponse {
    id: String,
    output: Option<Vec<ResponseItem>>,
    output_text: Option<String>,
    error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponseItem {
    #[serde(rename = "type")]
    kind: Option<String>,
    name: Option<String>,
    arguments: Option<String>,
    call_id: Option<String>,
    content: Option<Vec<ResponseContent>>,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    #[serde(rename = "type")]
    kind: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Serialize)]
struct ResponsesRequest<'a> {
    model: &'a str,
    input: Value,
    tools: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    previous_response_id: Option<&'a str>,
    parallel_tool_calls: bool,
}

#[derive(Debug)]
struct PendingToolCall {
    name: String,
    arguments: Value,
    call_id: String,
}

struct ModelTurn {
    response_id: String,
    output_text: String,
    tool_calls: Vec<PendingToolCall>,
}

pub async fn run_research(
    client: &Client,
    prompt: &str,
    model_override: Option<&str>,
    max_steps: usize,
    mut progress: Option<&mut dyn FnMut(ResearchProgress)>,
) -> Result<ResearchResult, CliError> {
    if max_steps == 0 {
        return Err(CliError::General(
            "max_steps must be at least 1 for research runs".into(),
        ));
    }

    let model = model_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| std::env::var("OPENAI_MODEL").ok())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());

    let openai_api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
        CliError::Config("OPENAI_API_KEY is required for `skyfi research`".to_string())
    })?;
    let openai_base_url = std::env::var("OPENAI_BASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_OPENAI_BASE_URL.to_string());

    let tool_schemas = research_tool_schemas();
    let mut previous_response_id: Option<String> = None;
    let mut next_input = json!([
        {
            "role": "user",
            "content": prompt,
        }
    ]);
    let mut tool_calls = Vec::new();

    for step in 1..=max_steps {
        let turn = if let Some(report) = progress.as_mut() {
            report(ResearchProgress::ModelRoundStarted { step });
            create_response(
                &openai_api_key,
                &openai_base_url,
                &model,
                next_input,
                &tool_schemas,
                previous_response_id.as_deref(),
                Some(&mut **report),
            )
            .await?
        } else {
            create_response(
                &openai_api_key,
                &openai_base_url,
                &model,
                next_input,
                &tool_schemas,
                previous_response_id.as_deref(),
                None,
            )
            .await?
        };

        let response_id = turn.response_id.clone();
        let calls = turn.tool_calls;
        if calls.is_empty() {
            let markdown = if turn.output_text.trim().is_empty() {
                return Err(CliError::General(
                    "research loop finished without terminal markdown output".into(),
                ));
            } else {
                turn.output_text.trim().to_string()
            };
            return Ok(ResearchResult {
                markdown,
                response_id,
                model,
                tool_calls,
            });
        }

        let mut outputs = Vec::with_capacity(calls.len());
        for call in calls {
            if let Some(report) = progress.as_mut() {
                report(ResearchProgress::ToolCallRequested {
                    step,
                    name: call.name.clone(),
                });
            }
            let result = execute_tool(client, &call.name, &call.arguments).await;
            let (output_value, is_error) = match result {
                Ok(value) => (value, false),
                Err(error) => (
                    json!({
                        "error": error.to_string(),
                    }),
                    true,
                ),
            };
            tool_calls.push(ResearchToolCall {
                step,
                name: call.name.clone(),
                arguments: call.arguments.clone(),
                output: output_value.clone(),
                is_error,
            });
            if let Some(report) = progress.as_mut() {
                report(ResearchProgress::ToolResult {
                    step,
                    name: call.name.clone(),
                    is_error,
                });
            }
            outputs.push(json!({
                "type": "function_call_output",
                "call_id": call.call_id,
                "output": output_value.to_string(),
            }));
        }

        previous_response_id = Some(response_id);
        next_input = Value::Array(outputs);
    }

    Err(CliError::General(format!(
        "research loop reached max_steps ({max_steps}) without producing a terminal answer"
    )))
}

pub fn render_report(prompt: &str, result: &ResearchResult) -> String {
    let mut rendered = String::new();
    rendered.push_str("# SkyFi Research Brief\n\n");
    rendered.push_str("## Objective\n\n");
    rendered.push_str(prompt.trim());
    rendered.push_str("\n\n");
    rendered.push_str("## Analysis\n\n");
    rendered.push_str(result.markdown.trim());
    rendered.push_str("\n\n");
    rendered.push_str("## Run Metadata\n\n");
    rendered.push_str(&format!("- Generated at: {}\n", Utc::now().to_rfc3339()));
    rendered.push_str(&format!("- Model: {}\n", result.model));
    rendered.push_str(&format!("- Response ID: {}\n", result.response_id));
    rendered.push_str(&format!("- Tool calls: {}\n", result.tool_calls.len()));

    if !result.tool_calls.is_empty() {
        rendered.push_str("\n## Tool Trace\n\n");
        for call in &result.tool_calls {
            rendered.push_str(&format!(
                "### {}. `{}`{}\n\n",
                call.step,
                call.name,
                if call.is_error { " (error)" } else { "" }
            ));
            rendered.push_str("**Arguments**\n\n```json\n");
            rendered.push_str(
                &serde_json::to_string_pretty(&call.arguments).unwrap_or_else(|_| "{}".to_string()),
            );
            rendered.push_str("\n```\n\n");
            rendered.push_str("**Output**\n\n```json\n");
            rendered.push_str(
                &serde_json::to_string_pretty(&call.output).unwrap_or_else(|_| "{}".to_string()),
            );
            rendered.push_str("\n```\n\n");
        }
    }

    rendered
}

pub fn default_report_path() -> PathBuf {
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    PathBuf::from(format!("skyfi-research-{timestamp}.md"))
}

pub fn artifact_from_paths(
    prompt: &str,
    result: &ResearchResult,
    markdown_path: &Path,
    trace_path: Option<&Path>,
) -> ResearchRunArtifact {
    ResearchRunArtifact {
        prompt: prompt.to_string(),
        model: result.model.clone(),
        generated_at: Utc::now().to_rfc3339(),
        markdown_path: markdown_path.display().to_string(),
        trace_path: trace_path.map(|path| path.display().to_string()),
        response_id: result.response_id.clone(),
        tool_calls: result.tool_calls.clone(),
    }
}

fn research_system_prompt() -> String {
    [
        "You are SkyFi Research Analyst, a geospatial investigation agent.",
        "Your job is to answer the user's objective by using the available tools, then produce a concise markdown brief.",
        "Use tools when they materially improve the answer. Do not invent coordinates, AOIs, archive IDs, feasibility outcomes, prices, or pass windows.",
        "If location resolution returns multiple plausible matches, pick the most relevant result unless the ambiguity materially changes the investigation.",
        "Never claim an imagery order was placed. This workflow is research-only and must stay read-only.",
        "The final answer must be markdown with these sections when relevant: Findings, Evidence, Gaps and Uncertainty, Recommended Next Actions.",
        "Be explicit about uncertainty when the available imagery, pricing, or feasibility evidence is incomplete.",
    ]
    .join(" ")
}

async fn create_response(
    api_key: &str,
    base_url: &str,
    model: &str,
    input: Value,
    tools: &[Value],
    previous_response_id: Option<&str>,
    progress: Option<&mut dyn FnMut(ResearchProgress)>,
) -> Result<ModelTurn, CliError> {
    if let Some(progress) = progress {
        return create_response_streaming(
            api_key,
            base_url,
            model,
            input,
            tools,
            previous_response_id,
            progress,
        )
        .await;
    }

    let url = Url::parse(base_url)
        .map_err(|error| CliError::Config(format!("invalid OPENAI_BASE_URL: {error}")))?
        .join("responses")
        .map_err(|error| CliError::Config(format!("invalid responses URL: {error}")))?;

    let body = ResponsesRequest {
        model,
        input,
        tools: tools.to_vec(),
        previous_response_id,
        parallel_tool_calls: false,
    };

    let response = reqwest::Client::new()
        .post(url)
        .bearer_auth(api_key)
        .json(&json!({
            "model": body.model,
            "input": body.input,
            "tools": body.tools,
            "previous_response_id": body.previous_response_id,
            "parallel_tool_calls": body.parallel_tool_calls,
            "instructions": research_system_prompt(),
        }))
        .send()
        .await?;

    let status = response.status();
    let payload: ResponsesApiResponse = response.json().await?;
    if !status.is_success() {
        return Err(CliError::Api {
            status: status.as_u16(),
            message: payload
                .error
                .and_then(|error| error.message)
                .unwrap_or_else(|| "Responses API request failed".to_string()),
        });
    }
    let response_id = payload.id.clone();
    Ok(ModelTurn {
        response_id,
        output_text: extract_output_text(&payload),
        tool_calls: extract_tool_calls(&payload)?,
    })
}

async fn create_response_streaming(
    api_key: &str,
    base_url: &str,
    model: &str,
    input: Value,
    tools: &[Value],
    previous_response_id: Option<&str>,
    progress: &mut dyn FnMut(ResearchProgress),
) -> Result<ModelTurn, CliError> {
    let url = Url::parse(base_url)
        .map_err(|error| CliError::Config(format!("invalid OPENAI_BASE_URL: {error}")))?
        .join("responses")
        .map_err(|error| CliError::Config(format!("invalid responses URL: {error}")))?;

    let response = reqwest::Client::new()
        .post(url)
        .bearer_auth(api_key)
        .json(&json!({
            "model": model,
            "input": input,
            "tools": tools,
            "previous_response_id": previous_response_id,
            "parallel_tool_calls": false,
            "instructions": research_system_prompt(),
            "stream": true,
        }))
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let payload: ResponsesApiResponse = response.json().await?;
        return Err(CliError::Api {
            status: status.as_u16(),
            message: payload
                .error
                .and_then(|error| error.message)
                .unwrap_or_else(|| "Responses API streaming request failed".to_string()),
        });
    }

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut response_id: Option<String> = None;
    let mut output_text = String::new();
    let mut tool_calls = Vec::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(index) = buffer.find("\n\n") {
            let raw_event = buffer[..index].to_string();
            buffer.drain(..index + 2);
            handle_sse_event(
                &raw_event,
                &mut response_id,
                &mut output_text,
                &mut tool_calls,
                progress,
            )?;
        }
    }

    if !buffer.trim().is_empty() {
        handle_sse_event(
            &buffer,
            &mut response_id,
            &mut output_text,
            &mut tool_calls,
            progress,
        )?;
    }

    Ok(ModelTurn {
        response_id: response_id.unwrap_or_else(|| "streaming-response".to_string()),
        output_text,
        tool_calls,
    })
}

fn handle_sse_event(
    raw_event: &str,
    response_id: &mut Option<String>,
    output_text: &mut String,
    tool_calls: &mut Vec<PendingToolCall>,
    progress: &mut dyn FnMut(ResearchProgress),
) -> Result<(), CliError> {
    let data = raw_event
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n");

    if data.is_empty() || data == "[DONE]" {
        return Ok(());
    }

    let event: Value = serde_json::from_str(&data)?;
    let event_type = event.get("type").and_then(Value::as_str).unwrap_or("");

    match event_type {
        "response.created" => {
            if let Some(id) = event
                .get("response")
                .and_then(|value| value.get("id"))
                .and_then(Value::as_str)
            {
                *response_id = Some(id.to_string());
            }
        }
        "response.output_text.delta" => {
            if let Some(delta) = event.get("delta").and_then(Value::as_str) {
                output_text.push_str(delta);
                progress(ResearchProgress::ModelTextDelta(delta.to_string()));
            }
        }
        "response.output_item.done" => {
            if let Some(item) = event.get("item") {
                maybe_collect_function_call(item, tool_calls)?;
            }
        }
        "response.function_call_arguments.done" => {
            if let Some(item) = event.get("item") {
                maybe_collect_function_call(item, tool_calls)?;
            }
        }
        "response.completed" => {
            if let Some(id) = event
                .get("response")
                .and_then(|value| value.get("id"))
                .and_then(Value::as_str)
            {
                *response_id = Some(id.to_string());
            }
        }
        "error" => {
            let message = event
                .get("error")
                .and_then(|value| value.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("Responses API stream returned an error event");
            return Err(CliError::General(message.to_string()));
        }
        _ => {}
    }

    Ok(())
}

fn maybe_collect_function_call(
    item: &Value,
    tool_calls: &mut Vec<PendingToolCall>,
) -> Result<(), CliError> {
    if item.get("type").and_then(Value::as_str) != Some("function_call") {
        return Ok(());
    }

    let name = item
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| CliError::General("function_call item missing name".into()))?;
    let call_id = item
        .get("call_id")
        .and_then(Value::as_str)
        .ok_or_else(|| CliError::General("function_call item missing call_id".into()))?;
    let arguments = item
        .get("arguments")
        .and_then(Value::as_str)
        .unwrap_or("{}");
    let parsed_arguments = serde_json::from_str::<Value>(arguments)?;

    if !tool_calls
        .iter()
        .any(|existing| existing.call_id == call_id)
    {
        tool_calls.push(PendingToolCall {
            name: name.to_string(),
            arguments: parsed_arguments,
            call_id: call_id.to_string(),
        });
    }

    Ok(())
}

fn extract_tool_calls(response: &ResponsesApiResponse) -> Result<Vec<PendingToolCall>, CliError> {
    let mut calls = Vec::new();
    for item in response.output.as_ref().into_iter().flatten() {
        if item.kind.as_deref() != Some("function_call") {
            continue;
        }
        let name = item
            .name
            .clone()
            .ok_or_else(|| CliError::General("function_call missing name".into()))?;
        let arguments = item.arguments.clone().unwrap_or_else(|| "{}".to_string());
        let call_id = item
            .call_id
            .clone()
            .ok_or_else(|| CliError::General("function_call missing call_id".into()))?;
        let parsed_arguments = serde_json::from_str::<Value>(&arguments)?;
        calls.push(PendingToolCall {
            name,
            arguments: parsed_arguments,
            call_id,
        });
    }
    Ok(calls)
}

fn extract_output_text(response: &ResponsesApiResponse) -> String {
    if let Some(text) = response
        .output_text
        .as_ref()
        .filter(|text| !text.trim().is_empty())
    {
        return text.trim().to_string();
    }

    let mut parts = Vec::new();
    for item in response.output.as_ref().into_iter().flatten() {
        for content in item.content.as_ref().into_iter().flatten() {
            if matches!(content.kind.as_deref(), Some("output_text") | Some("text")) {
                if let Some(text) = content.text.as_ref().filter(|text| !text.trim().is_empty()) {
                    parts.push(text.trim().to_string());
                }
            }
        }
    }

    parts.join("\n\n")
}

async fn execute_tool(client: &Client, name: &str, arguments: &Value) -> Result<Value, CliError> {
    match name {
        "account_whoami" => {
            let response = client.get("/auth/whoami").await?;
            Ok(response.json::<Value>().await?)
        }
        "resolve_location" => resolve_location(arguments).await,
        "archives_search" => {
            let args = object_arguments(arguments, "archives_search")?;
            let req = GetArchivesRequest {
                aoi: required_string(args, "aoi", "archives_search")?,
                from_date: optional_string(args, "from_date"),
                to_date: optional_string(args, "to_date"),
                max_cloud_coverage_percent: optional_f64(args, "max_cloud_coverage_percent")?,
                max_off_nadir_angle: optional_f64(args, "max_off_nadir_angle")?,
                resolutions: optional_string_list(args, "resolutions"),
                product_types: optional_product_types(args, "product_types")?,
                providers: None,
                open_data: optional_bool(args, "open_data"),
                min_overlap_ratio: optional_f64(args, "min_overlap_ratio")?,
                page_number: optional_i64(args, "page_number")?,
                page_size: optional_i64(args, "page_size")?.unwrap_or(10),
            };
            let response = client.post("/archives", &req).await?;
            Ok(response.json::<Value>().await?)
        }
        "archive_get" => {
            let args = object_arguments(arguments, "archive_get")?;
            let archive_id = required_string(args, "archive_id", "archive_get")?;
            let response = client.get(&format!("/archives/{archive_id}")).await?;
            Ok(response.json::<Value>().await?)
        }
        "pricing_get" => {
            let args = object_arguments(arguments, "pricing_get")?;
            let req = PricingRequest {
                aoi: optional_string(args, "aoi"),
            };
            let response = client.post("/pricing", &req).await?;
            Ok(response.json::<Value>().await?)
        }
        "feasibility_check" => {
            let args = object_arguments(arguments, "feasibility_check")?;
            let req = FeasibilityRequest {
                aoi: required_string(args, "aoi", "feasibility_check")?,
                product_type: required_product_type(args, "product_type", "feasibility_check")?,
                resolution: required_string(args, "resolution", "feasibility_check")?,
                start_date: required_string(args, "start_date", "feasibility_check")?,
                end_date: required_string(args, "end_date", "feasibility_check")?,
                max_cloud_coverage_percent: optional_f64(args, "max_cloud_coverage_percent")?,
                priority_item: optional_bool(args, "priority_item"),
                required_provider: optional_string(args, "required_provider"),
            };
            let response = client.post("/feasibility", &req).await?;
            Ok(response.json::<Value>().await?)
        }
        "passes_predict" => {
            let args = object_arguments(arguments, "passes_predict")?;
            let req = PassPredictionRequest {
                aoi: required_string(args, "aoi", "passes_predict")?,
                from_date: required_string(args, "from_date", "passes_predict")?,
                to_date: required_string(args, "to_date", "passes_predict")?,
                product_types: optional_product_types(args, "product_types")?,
                resolutions: optional_string_list(args, "resolutions"),
                max_off_nadir_angle: optional_f64(args, "max_off_nadir_angle")?,
            };
            let response = client.post("/feasibility/pass-prediction", &req).await?;
            Ok(response.json::<Value>().await?)
        }
        _ => Err(CliError::General(format!(
            "unknown research tool requested: {name}"
        ))),
    }
}

async fn resolve_location(arguments: &Value) -> Result<Value, CliError> {
    let args = object_arguments(arguments, "resolve_location")?;
    let query = required_string(args, "query", "resolve_location")?;

    let response = reqwest::Client::new()
        .get("https://nominatim.openstreetmap.org/search")
        .header(
            reqwest::header::USER_AGENT,
            "skyfi-cli research mode (https://github.com/ianzepp/skyfi-cli)",
        )
        .query(&[
            ("q", query.clone()),
            ("format", "jsonv2".to_string()),
            ("limit", "5".to_string()),
        ])
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(CliError::Api {
            status: response.status().as_u16(),
            message: format!("OpenStreetMap lookup failed for query: {query}"),
        });
    }

    let results = response.json::<Vec<Value>>().await?;
    let normalized = results
        .into_iter()
        .filter_map(|item| {
            let display_name = item.get("display_name")?.as_str()?.to_string();
            let lat = item.get("lat")?.as_str()?.parse::<f64>().ok()?;
            let lon = item.get("lon")?.as_str()?.parse::<f64>().ok()?;
            let bbox = item.get("boundingbox")?.as_array()?;
            if bbox.len() != 4 {
                return None;
            }
            let south = bbox[0].as_str()?.parse::<f64>().ok()?;
            let north = bbox[1].as_str()?.parse::<f64>().ok()?;
            let west = bbox[2].as_str()?.parse::<f64>().ok()?;
            let east = bbox[3].as_str()?.parse::<f64>().ok()?;
            Some(json!({
                "name": display_name,
                "lat": lat,
                "lon": lon,
                "bounding_box": {
                    "south": south,
                    "north": north,
                    "west": west,
                    "east": east,
                },
                "wkt_polygon": bbox_to_wkt(south, north, west, east),
                "type": item.get("type").and_then(Value::as_str),
            }))
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "query": query,
        "results": normalized,
    }))
}

fn bbox_to_wkt(south: f64, north: f64, west: f64, east: f64) -> String {
    format!(
        "POLYGON (({west} {south}, {east} {south}, {east} {north}, {west} {north}, {west} {south}))"
    )
}

fn object_arguments<'a>(
    value: &'a Value,
    tool_name: &str,
) -> Result<&'a serde_json::Map<String, Value>, CliError> {
    value.as_object().ok_or_else(|| {
        CliError::General(format!(
            "{tool_name} expected object arguments but received: {value}"
        ))
    })
}

fn required_string(
    args: &serde_json::Map<String, Value>,
    key: &str,
    tool_name: &str,
) -> Result<String, CliError> {
    args.get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| CliError::General(format!("{tool_name} requires string field `{key}`")))
}

fn optional_string(args: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
}

fn optional_bool(args: &serde_json::Map<String, Value>, key: &str) -> Option<bool> {
    args.get(key).and_then(Value::as_bool)
}

fn optional_i64(args: &serde_json::Map<String, Value>, key: &str) -> Result<Option<i64>, CliError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .map(Some)
            .ok_or_else(|| CliError::General(format!("expected integer field `{key}`"))),
    }
}

fn optional_f64(args: &serde_json::Map<String, Value>, key: &str) -> Result<Option<f64>, CliError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_f64()
            .map(Some)
            .ok_or_else(|| CliError::General(format!("expected numeric field `{key}`"))),
    }
}

fn optional_string_list(args: &serde_json::Map<String, Value>, key: &str) -> Option<Vec<String>> {
    args.get(key).and_then(Value::as_array).map(|items| {
        items
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    })
}

fn optional_product_types(
    args: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<Vec<ProductType>>, CliError> {
    let Some(values) = args.get(key) else {
        return Ok(None);
    };
    let Some(array) = values.as_array() else {
        return Err(CliError::General(format!(
            "expected `{key}` to be an array of product types"
        )));
    };
    let mut parsed = Vec::with_capacity(array.len());
    for value in array {
        let Some(raw) = value.as_str() else {
            return Err(CliError::General(format!(
                "expected `{key}` items to be strings"
            )));
        };
        parsed.push(parse_product_type(raw)?);
    }
    Ok(Some(parsed))
}

fn required_product_type(
    args: &serde_json::Map<String, Value>,
    key: &str,
    tool_name: &str,
) -> Result<ProductType, CliError> {
    let raw = required_string(args, key, tool_name)?;
    parse_product_type(&raw)
}

fn parse_product_type(raw: &str) -> Result<ProductType, CliError> {
    match raw.to_ascii_uppercase().as_str() {
        "DAY" => Ok(ProductType::Day),
        "NIGHT" => Ok(ProductType::Night),
        "VIDEO" => Ok(ProductType::Video),
        "SAR" => Ok(ProductType::Sar),
        "HYPERSPECTRAL" => Ok(ProductType::Hyperspectral),
        "MULTISPECTRAL" => Ok(ProductType::Multispectral),
        "STEREO" => Ok(ProductType::Stereo),
        "BASEMAP" => Ok(ProductType::Basemap),
        _ => Err(CliError::General(format!(
            "unsupported product type: {raw}"
        ))),
    }
}

fn research_tool_schemas() -> Vec<Value> {
    vec![
        json!({
            "type": "function",
            "name": "account_whoami",
            "description": "Get SkyFi account readiness, budget usage, and payment readiness.",
            "parameters": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        }),
        json!({
            "type": "function",
            "name": "resolve_location",
            "description": "Resolve a place name or address to coordinates and a WKT polygon using OpenStreetMap.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" }
                },
                "required": ["query"],
                "additionalProperties": false
            }
        }),
        json!({
            "type": "function",
            "name": "archives_search",
            "description": "Search archive imagery over an AOI using WKT geometry and optional filters.",
            "parameters": {
                "type": "object",
                "properties": {
                    "aoi": { "type": "string" },
                    "from_date": { "type": "string" },
                    "to_date": { "type": "string" },
                    "max_cloud_coverage_percent": { "type": "number" },
                    "max_off_nadir_angle": { "type": "number" },
                    "resolutions": { "type": "array", "items": { "type": "string" } },
                    "product_types": { "type": "array", "items": { "type": "string" } },
                    "open_data": { "type": "boolean" },
                    "min_overlap_ratio": { "type": "number" },
                    "page_number": { "type": "integer" },
                    "page_size": { "type": "integer" }
                },
                "required": ["aoi"],
                "additionalProperties": false
            }
        }),
        json!({
            "type": "function",
            "name": "archive_get",
            "description": "Get detail for a specific archive image by archive ID.",
            "parameters": {
                "type": "object",
                "properties": {
                    "archive_id": { "type": "string" }
                },
                "required": ["archive_id"],
                "additionalProperties": false
            }
        }),
        json!({
            "type": "function",
            "name": "pricing_get",
            "description": "Get provider pricing, optionally scoped to an AOI.",
            "parameters": {
                "type": "object",
                "properties": {
                    "aoi": { "type": "string" }
                },
                "additionalProperties": false
            }
        }),
        json!({
            "type": "function",
            "name": "feasibility_check",
            "description": "Check whether a new collection is feasible for an AOI and time window.",
            "parameters": {
                "type": "object",
                "properties": {
                    "aoi": { "type": "string" },
                    "product_type": { "type": "string" },
                    "resolution": { "type": "string" },
                    "start_date": { "type": "string" },
                    "end_date": { "type": "string" },
                    "max_cloud_coverage_percent": { "type": "number" },
                    "priority_item": { "type": "boolean" },
                    "required_provider": { "type": "string" }
                },
                "required": ["aoi", "product_type", "resolution", "start_date", "end_date"],
                "additionalProperties": false
            }
        }),
        json!({
            "type": "function",
            "name": "passes_predict",
            "description": "Predict upcoming satellite passes for an AOI and date window.",
            "parameters": {
                "type": "object",
                "properties": {
                    "aoi": { "type": "string" },
                    "from_date": { "type": "string" },
                    "to_date": { "type": "string" },
                    "product_types": { "type": "array", "items": { "type": "string" } },
                    "resolutions": { "type": "array", "items": { "type": "string" } },
                    "max_off_nadir_angle": { "type": "number" }
                },
                "required": ["aoi", "from_date", "to_date"],
                "additionalProperties": false
            }
        }),
    ]
}

#[cfg(test)]
#[path = "research_test.rs"]
mod tests;
