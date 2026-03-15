use crate::client::Client;
use crate::error::CliError;
use crate::output;
use crate::research::{
    artifact_from_paths, default_report_path, render_report, run_research, ResearchProgress,
};
use std::fs;
use std::path::Path;

pub async fn run(
    client: &Client,
    prompt: &str,
    output_path: Option<&Path>,
    trace_output_path: Option<&Path>,
    model: Option<&str>,
    max_steps: usize,
    json: bool,
) -> Result<(), CliError> {
    let mut text_stream_open = false;
    let mut reporter = |event: ResearchProgress| match event {
        ResearchProgress::ModelRoundStarted { step } => {
            if text_stream_open {
                eprintln!();
                text_stream_open = false;
            }
            eprintln!("== Step {step}: model ==");
        }
        ResearchProgress::ModelTextDelta(delta) => {
            eprint!("{delta}");
            text_stream_open = true;
        }
        ResearchProgress::ToolCallRequested { step, name } => {
            if text_stream_open {
                eprintln!();
                text_stream_open = false;
            }
            eprintln!("-> step {step}: calling `{name}`");
        }
        ResearchProgress::ToolResult {
            step,
            name,
            is_error,
        } => {
            eprintln!(
                "<- step {step}: `{name}` {}",
                if is_error { "failed" } else { "completed" }
            );
        }
    };

    let progress = if json {
        None
    } else {
        Some(&mut reporter as &mut dyn FnMut(ResearchProgress))
    };

    let result = run_research(client, prompt, model, max_steps, progress).await?;
    if text_stream_open {
        eprintln!();
    }
    let report = render_report(prompt, &result);
    let markdown_path = output_path
        .map(Path::to_path_buf)
        .unwrap_or_else(default_report_path);

    if let Some(parent) = markdown_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(&markdown_path, report)?;

    if let Some(trace_path) = trace_output_path {
        if let Some(parent) = trace_path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        let artifact = artifact_from_paths(prompt, &result, &markdown_path, Some(trace_path));
        fs::write(trace_path, serde_json::to_string_pretty(&artifact)?)?;
    }

    if json {
        let artifact = artifact_from_paths(prompt, &result, &markdown_path, trace_output_path);
        output::print_json(&artifact)?;
    } else {
        println!("Research brief written to {}", markdown_path.display());
        if let Some(trace_path) = trace_output_path {
            println!("Trace written to {}", trace_path.display());
        }
    }

    Ok(())
}
