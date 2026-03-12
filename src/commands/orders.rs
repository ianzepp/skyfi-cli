//! Order creation, listing, retrieval, and delivery commands.
//!
//! Orders are the mechanism through which imagery is purchased or commissioned.
//! There are two order types:
//!
//! - **Archive orders** — purchase an existing image from the catalog. Charged
//!   based on the AOI area clipped from the archive scene.
//! - **Tasking orders** — commission a new satellite capture within a future
//!   time window. The satellite provider schedules the capture; delivery occurs
//!   after the capture and processing pipeline completes.
//!
//! # Commands
//!
//! - `orders list` — GET `/orders` with pagination and sort parameters.
//! - `orders get <ID>` — GET `/orders/{id}`; full detail and status history.
//! - `orders order-archive` — POST `/order-archive`.
//! - `orders order-tasking` — POST `/order-tasking`.
//! - `orders pass-targeted` — Composite: calls `pass-prediction` then `order-tasking`.
//! - `orders download <ID>` — GET `/orders/{id}/{type}`; returns the redirect URL.
//! - `orders redeliver <ID>` — POST `/orders/{id}/redelivery`.
//!
//! # Pass-targeted workflow
//!
//! The `PassTargeted` action is a multi-step composite command that exists to
//! reduce the number of round-trips for a common use case: find the next
//! satellite pass and immediately book it. The two-step manual equivalent is:
//!
//! 1. `feasibility pass-prediction` → get a list of passes with `providerWindowId` values
//! 2. `orders order-tasking --provider-window-id <UUID>` → lock the order to that pass
//!
//! TRADE-OFF: Compositing these into one command means the user cannot inspect the
//! pass list before committing to the order. Supply `--provider-window-id` explicitly
//! to override auto-selection when you need that control.

use crate::cli::OrdersAction;
use crate::client::Client;
use crate::error::CliError;
use crate::output;
use crate::types::*;
use chrono::{DateTime, NaiveDate};
use serde::{Deserialize, Serialize};

/// Serialize an enum value to its string representation for use as a URL query parameter.
///
/// WHY: `reqwest`'s `.query()` method serializes structs as form-encoded key-value pairs,
/// but enum serialization via serde can produce JSON strings with surrounding quotes. This
/// helper extracts the raw string value so the query parameter arrives correctly.
fn enum_query_value<T: serde::Serialize>(value: &T, field_name: &str) -> Result<String, CliError> {
    let value = serde_json::to_value(value)?;
    value
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| CliError::General(format!("{field_name} did not serialize to a string")))
}

pub async fn run(action: OrdersAction, client: &Client, json: bool) -> Result<(), CliError> {
    match action {
        OrdersAction::List {
            order_type,
            sort_by,
            sort_dir,
            page,
            page_size,
        } => {
            let mut query: Vec<(&str, String)> = vec![];
            if let Some(ref ot) = order_type {
                query.push(("orderType", format!("{ot:?}").to_uppercase()));
            }
            if let Some(ref cols) = sort_by {
                for c in cols {
                    query.push(("sort_columns", enum_query_value(c, "sort column")?));
                }
            }
            if let Some(ref dirs) = sort_dir {
                for d in dirs {
                    query.push(("sort_directions", enum_query_value(d, "sort direction")?));
                }
            }
            if let Some(p) = page {
                query.push(("pageNumber", p.to_string()));
            }
            query.push(("pageSize", page_size.to_string()));

            let resp = client.get_query("/orders", &query).await?;
            let data: ListOrdersResponse = resp.json().await?;
            if json {
                output::print_json(&data)?;
            } else {
                eprintln!("Total orders: {}", data.total);
                for order in &data.orders {
                    println!(
                        "{:<36}  {:>10}  {:?}  {}",
                        order.order_id,
                        format!("{:?}", order.status),
                        order.order_type,
                        order.created_at,
                    );
                }
            }
        }
        OrdersAction::Get { order_id } => {
            let resp = client.get(&format!("/orders/{order_id}")).await?;
            let data: serde_json::Value = resp.json().await?;
            if json {
                output::print_json(&data)?;
            } else {
                output::print_value(&data, 0);
            }
        }
        OrdersAction::OrderArchive {
            aoi,
            archive_id,
            label,
            delivery_driver,
            webhook_url,
        } => {
            let req = ArchiveOrderRequest {
                aoi,
                archive_id,
                label: label.clone(),
                order_label: label,
                delivery_driver,
                delivery_params: None,
                webhook_url,
                metadata: None,
            };
            let resp = client.post("/order-archive", &req).await?;
            let data: serde_json::Value = resp.json().await?;
            if json {
                output::print_json(&data)?;
            } else {
                println!("Order created:");
                println!(
                    "  ID:     {}",
                    data.get("orderId").and_then(|v| v.as_str()).unwrap_or("-")
                );
                println!(
                    "  Code:   {}",
                    data.get("orderCode")
                        .and_then(|v| v.as_str())
                        .unwrap_or("-")
                );
                println!(
                    "  Status: {}",
                    data.get("status").and_then(|v| v.as_str()).unwrap_or("-")
                );
                println!(
                    "  Cost:   {} cents",
                    data.get("orderCost").and_then(|v| v.as_i64()).unwrap_or(0)
                );
            }
        }
        OrdersAction::OrderTasking {
            aoi,
            window_start,
            window_end,
            product_type,
            resolution,
            label,
            priority,
            max_cloud,
            max_nadir,
            required_provider,
            delivery_driver,
            webhook_url,
            provider_window_id,
        } => {
            let req = TaskingOrderRequest {
                aoi,
                window_start,
                window_end,
                product_type,
                resolution,
                label: label.clone(),
                order_label: label,
                priority_item: priority,
                max_cloud_coverage_percent: max_cloud,
                max_off_nadir_angle: max_nadir,
                required_provider,
                delivery_driver,
                delivery_params: None,
                webhook_url,
                metadata: None,
                sar_product_types: None,
                sar_polarisation: None,
                provider_window_id,
            };
            let resp = client.post("/order-tasking", &req).await?;
            let data: serde_json::Value = resp.json().await?;
            if json {
                output::print_json(&data)?;
            } else {
                println!("Tasking order created:");
                println!(
                    "  ID:     {}",
                    data.get("orderId").and_then(|v| v.as_str()).unwrap_or("-")
                );
                println!(
                    "  Code:   {}",
                    data.get("orderCode")
                        .and_then(|v| v.as_str())
                        .unwrap_or("-")
                );
                println!(
                    "  Status: {}",
                    data.get("status").and_then(|v| v.as_str()).unwrap_or("-")
                );
            }
        }
        OrdersAction::PassTargeted {
            aoi,
            window_start,
            window_end,
            product_type,
            resolution,
            label,
            priority,
            max_cloud,
            max_nadir,
            required_provider,
            delivery_driver,
            webhook_url,
            provider_window_id,
        } => {
            let from_date = prediction_date(&window_start)?;
            let to_date = prediction_date(&window_end)?;
            let prediction_request = PassPredictionRequest {
                aoi: aoi.clone(),
                from_date,
                to_date,
                product_types: Some(vec![product_type.clone()]),
                resolutions: Some(vec![resolution.clone()]),
                max_off_nadir_angle: max_nadir.map(|value| value as f64),
            };

            let prediction_response = client
                .post("/feasibility/pass-prediction", &prediction_request)
                .await?;
            let predicted_passes: PassPredictionResponse = prediction_response.json().await?;
            let passes = deserialize_passes(predicted_passes.passes)?;
            let selected_pass = select_pass(&passes, provider_window_id.as_deref())?;

            let tasking_request = TaskingOrderRequest {
                aoi,
                window_start,
                window_end,
                product_type,
                resolution,
                label: label.clone(),
                order_label: label,
                priority_item: priority,
                max_cloud_coverage_percent: max_cloud,
                max_off_nadir_angle: max_nadir,
                required_provider,
                delivery_driver,
                delivery_params: None,
                webhook_url,
                metadata: None,
                sar_product_types: None,
                sar_polarisation: None,
                provider_window_id: Some(selected_pass.provider_window_id.clone()),
            };

            let order_response = client.post("/order-tasking", &tasking_request).await?;
            let order_data: serde_json::Value = order_response.json().await?;

            if json {
                output::print_json(&serde_json::json!({
                    "selectedPass": selected_pass,
                    "order": order_data
                }))?;
            } else {
                print_selected_pass(selected_pass);
                print_tasking_order(&order_data);
            }
        }
        OrdersAction::Download {
            order_id,
            deliverable_type,
        } => {
            let dtype = format!("{deliverable_type:?}").to_lowercase();
            let resp = client.get(&format!("/orders/{order_id}/{dtype}")).await?;
            // This endpoint redirects to a download URL
            let url = resp.url().to_string();
            if json {
                output::print_json(&serde_json::json!({ "download_url": url }))?;
            } else {
                println!("{url}");
            }
        }
        OrdersAction::Redeliver {
            order_id,
            delivery_driver,
            delivery_params,
        } => {
            let params: std::collections::HashMap<String, serde_json::Value> =
                serde_json::from_str(&delivery_params)
                    .map_err(|e| CliError::General(format!("invalid delivery params JSON: {e}")))?;
            let req = OrderRedeliveryRequest {
                delivery_driver,
                delivery_params: params,
            };
            let resp = client
                .post(&format!("/orders/{order_id}/redelivery"), &req)
                .await?;
            let data: serde_json::Value = resp.json().await?;
            if json {
                output::print_json(&data)?;
            } else {
                println!("Redelivery scheduled for order {order_id}");
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PredictedPass {
    provider_window_id: String,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    resolution: Option<String>,
    #[serde(default)]
    off_nadir_angle: Option<f64>,
    #[serde(default)]
    pass_date: Option<String>,
}

/// Extract the date portion from an ISO 8601 date-or-datetime string.
///
/// The `pass-prediction` API requires plain dates (`YYYY-MM-DD`), but
/// `orders pass-targeted` accepts datetime strings for its window arguments
/// (to match `order-tasking` expectations). This function normalizes either
/// format to the date-only form expected by the prediction endpoint.
fn prediction_date(value: &str) -> Result<String, CliError> {
    if let Ok(date_time) = DateTime::parse_from_rfc3339(value) {
        return Ok(date_time.format("%Y-%m-%d").to_string());
    }

    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        return Ok(date.format("%Y-%m-%d").to_string());
    }

    Err(CliError::General(format!(
        "invalid date/time '{value}'. Expected ISO 8601 date or date-time"
    )))
}

/// Deserialize the raw pass JSON values into typed `PredictedPass` structs.
///
/// WHY: `PassPredictionResponse` stores passes as `Vec<serde_json::Value>` because
/// the pass schema includes provider-specific fields that vary. We deserialize here
/// to get type-safe access to the fields we need for pass selection.
fn deserialize_passes(passes: Vec<serde_json::Value>) -> Result<Vec<PredictedPass>, CliError> {
    passes
        .into_iter()
        .map(|pass| serde_json::from_value(pass).map_err(CliError::from))
        .collect()
}

/// Select a pass from the predicted pass list.
///
/// When `provider_window_id` is provided, the pass with that exact ID is located
/// and returned (error if not found). When it is `None`, the earliest pass by
/// `pass_date` is selected. String comparison works here because pass dates are
/// ISO 8601 (`YYYY-MM-DD`), which sorts lexicographically in chronological order.
fn select_pass<'a>(
    passes: &'a [PredictedPass],
    provider_window_id: Option<&str>,
) -> Result<&'a PredictedPass, CliError> {
    if passes.is_empty() {
        return Err(CliError::General(
            "no matching passes found for pass-targeted tasking".into(),
        ));
    }

    if let Some(provider_window_id) = provider_window_id {
        return passes
            .iter()
            .find(|pass| pass.provider_window_id == provider_window_id)
            .ok_or_else(|| {
                CliError::General(format!(
                    "providerWindowId '{provider_window_id}' was not found in predicted passes"
                ))
            });
    }

    passes
        .iter()
        .min_by(|left, right| left.pass_date.cmp(&right.pass_date))
        .ok_or_else(|| CliError::General("no predicted passes available".into()))
}

fn print_selected_pass(pass: &PredictedPass) {
    println!("Selected pass:");
    println!(
        "  Provider:          {}",
        pass.provider.as_deref().unwrap_or("-")
    );
    println!(
        "  Resolution:        {}",
        pass.resolution.as_deref().unwrap_or("-")
    );
    println!(
        "  Off-nadir angle:   {}",
        pass.off_nadir_angle
            .map(|value| format!("{value:.1}°"))
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "  Pass date:         {}",
        pass.pass_date.as_deref().unwrap_or("-")
    );
    println!("  Provider window:   {}", pass.provider_window_id);
}

fn print_tasking_order(data: &serde_json::Value) {
    println!("Tasking order created:");
    println!(
        "  ID:     {}",
        data.get("orderId").and_then(|v| v.as_str()).unwrap_or("-")
    );
    println!(
        "  Code:   {}",
        data.get("orderCode")
            .and_then(|v| v.as_str())
            .unwrap_or("-")
    );
    println!(
        "  Status: {}",
        data.get("status").and_then(|v| v.as_str()).unwrap_or("-")
    );
}

#[cfg(test)]
#[path = "orders_test.rs"]
mod tests;
