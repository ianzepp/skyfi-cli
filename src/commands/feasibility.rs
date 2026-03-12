//! Feasibility check and satellite pass prediction commands.
//!
//! Feasibility analysis answers "can we capture this AOI at this resolution in
//! this time window?" by combining two signals:
//! - **Weather forecast probability** — cloud cover predictions for the date range
//! - **Satellite provider availability** — which providers have assets that can
//!   reach the AOI with the requested resolution in the window
//!
//! The result is an `overallScore` between 0 and 1, where 1 means high confidence
//! in a successful capture.
//!
//! # Commands
//!
//! - `feasibility check` — POST `/feasibility`; starts an async job, returns a task ID.
//!   The `--wait` flag turns this into a synchronous call by polling until complete.
//! - `feasibility status <ID>` — GET `/feasibility/{id}`; poll the job state.
//! - `feasibility pass-prediction` — POST `/feasibility/pass-prediction`; predict
//!   specific satellite overpasses within a date range.
//!
//! # Why feasibility is asynchronous
//!
//! Feasibility scoring requires querying external satellite provider scheduling
//! systems and weather APIs, which can take several seconds. The async design
//! lets the API return quickly with a task ID that the caller can poll or ignore,
//! rather than holding an HTTP connection open for the full computation.
//!
//! The `--wait` flag on `feasibility check` wraps the async API in a synchronous
//! polling loop (2-second interval) for callers that want a single blocking call.

use crate::cli::FeasibilityAction;
use crate::client::Client;
use crate::error::CliError;
use crate::output;
use crate::types::*;
use tokio::time::{sleep, Duration};

/// Dispatch a feasibility subcommand to the appropriate API call and render the output.
pub async fn run(action: FeasibilityAction, client: &Client, json: bool) -> Result<(), CliError> {
    match action {
        FeasibilityAction::Check {
            aoi,
            product_type,
            resolution,
            start_date,
            end_date,
            max_cloud,
            priority,
            required_provider,
            wait,
        } => {
            let req = FeasibilityRequest {
                aoi,
                product_type,
                resolution,
                start_date,
                end_date,
                max_cloud_coverage_percent: max_cloud,
                priority_item: priority,
                required_provider,
            };
            let resp = client.post("/feasibility", &req).await?;
            let data: FeasibilityTaskResponse = resp.json().await?;
            if wait {
                let final_status = wait_for_feasibility(client, &data.id).await?;
                if json {
                    output::print_json(&serde_json::json!({
                        "task": data,
                        "result": final_status
                    }))?;
                } else {
                    println!("Feasibility task created:");
                    println!("  ID:          {}", data.id);
                    println!("  Valid until: {}", data.valid_until);
                    if let Some(score) = &data.overall_score {
                        println!("  Score:       {score}");
                    }
                    println!("Final status:");
                    output::print_value(&final_status, 1);
                }
            } else if json {
                output::print_json(&data)?;
            } else {
                println!("Feasibility task created:");
                println!("  ID:          {}", data.id);
                println!("  Valid until: {}", data.valid_until);
                if let Some(score) = &data.overall_score {
                    println!("  Score:       {score}");
                }
            }
        }
        FeasibilityAction::Status { feasibility_id } => {
            let resp = client
                .get(&format!("/feasibility/{feasibility_id}"))
                .await?;
            let data: serde_json::Value = resp.json().await?;
            if json {
                output::print_json(&data)?;
            } else {
                output::print_value(&data, 0);
            }
        }
        FeasibilityAction::PassPrediction {
            aoi,
            from_date,
            to_date,
            product_types,
            resolutions,
            max_nadir,
        } => {
            let req = PassPredictionRequest {
                aoi,
                from_date,
                to_date,
                product_types,
                resolutions,
                max_off_nadir_angle: max_nadir,
            };
            let resp = client.post("/feasibility/pass-prediction", &req).await?;
            let data: PassPredictionResponse = resp.json().await?;
            if json {
                output::print_json(&data)?;
            } else {
                println!("Predicted passes: {}", data.passes.len());
                for pass in &data.passes {
                    if let Some(obj) = pass.as_object() {
                        println!(
                            "  {} {:>8} {:>6.1}° {}  {}",
                            obj.get("provider").and_then(|v| v.as_str()).unwrap_or("-"),
                            obj.get("resolution")
                                .and_then(|v| v.as_str())
                                .unwrap_or("-"),
                            obj.get("offNadirAngle")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0),
                            obj.get("passDate").and_then(|v| v.as_str()).unwrap_or("-"),
                            obj.get("providerWindowId")
                                .and_then(|v| v.as_str())
                                .unwrap_or("-"),
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

/// Poll a feasibility task until it reaches a terminal state (`COMPLETE` or `ERROR`).
///
/// Polls every 2 seconds. Returns the full final status JSON on `COMPLETE`,
/// or a `CliError::General` on `ERROR`. All intermediate states (`PENDING`,
/// `STARTED`, or unknown) continue polling.
async fn wait_for_feasibility(
    client: &Client,
    feasibility_id: &str,
) -> Result<serde_json::Value, CliError> {
    loop {
        let resp = client
            .get(&format!("/feasibility/{feasibility_id}"))
            .await?;
        let data: serde_json::Value = resp.json().await?;

        match feasibility_status(&data) {
            Some("COMPLETE") => return Ok(data),
            Some("ERROR") => {
                return Err(CliError::General(format!(
                    "feasibility check {feasibility_id} ended in ERROR"
                )))
            }
            Some(_) | None => sleep(Duration::from_secs(2)).await,
        }
    }
}

/// Extract the `status` string from a feasibility status response body.
fn feasibility_status(value: &serde_json::Value) -> Option<&str> {
    value.get("status").and_then(|status| status.as_str())
}

#[cfg(test)]
#[path = "feasibility_test.rs"]
mod tests;
