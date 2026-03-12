use crate::cli::FeasibilityAction;
use crate::client::Client;
use crate::error::CliError;
use crate::output;
use crate::types::*;

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
        } => {
            let aoi_value: serde_json::Value = serde_json::from_str(&aoi)
                .map_err(|e| CliError::General(format!("invalid AOI GeoJSON: {e}")))?;
            let req = FeasibilityRequest {
                aoi: aoi_value,
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
            if json {
                output::print_json(&data);
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
                println!("{}", serde_json::to_string_pretty(&data).unwrap());
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
            let aoi_value: serde_json::Value = serde_json::from_str(&aoi)
                .map_err(|e| CliError::General(format!("invalid AOI GeoJSON: {e}")))?;
            let req = PassPredictionRequest {
                aoi: aoi_value,
                from_date,
                to_date,
                product_types,
                resolutions,
                max_off_nadir_angle: max_nadir,
            };
            let resp = client.post("/feasibility/pass-prediction", &req).await?;
            let data: PassPredictionResponse = resp.json().await?;
            if json {
                output::print_json(&data);
            } else {
                println!("Predicted passes: {}", data.passes.len());
                for pass in &data.passes {
                    if let Some(obj) = pass.as_object() {
                        println!(
                            "  {} {:>8} {:>6.1}° {}",
                            obj.get("provider")
                                .and_then(|v| v.as_str())
                                .unwrap_or("-"),
                            obj.get("resolution")
                                .and_then(|v| v.as_str())
                                .unwrap_or("-"),
                            obj.get("offNadirAngle")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0),
                            obj.get("passDate")
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
