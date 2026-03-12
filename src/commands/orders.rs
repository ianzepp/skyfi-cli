use crate::cli::OrdersAction;
use crate::client::Client;
use crate::error::CliError;
use crate::output;
use crate::types::*;

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
                    query.push(("sort_columns", serde_json::to_string(c).unwrap().trim_matches('"').to_string()));
                }
            }
            if let Some(ref dirs) = sort_dir {
                for d in dirs {
                    query.push(("sort_directions", serde_json::to_string(d).unwrap().trim_matches('"').to_string()));
                }
            }
            if let Some(p) = page {
                query.push(("pageNumber", p.to_string()));
            }
            query.push(("pageSize", page_size.to_string()));

            let resp = client.get_query("/orders", &query).await?;
            let data: ListOrdersResponse = resp.json().await?;
            if json {
                output::print_json(&data);
            } else {
                eprintln!("Total orders: {}", data.total);
                for order in &data.orders {
                    let info = &order.order_info;
                    println!(
                        "{:<36}  {:>10}  {:?}  {}",
                        info.get("orderId").and_then(|v| v.as_str()).unwrap_or("-"),
                        format!("{:?}", order.event.status),
                        info.get("orderType")
                            .and_then(|v| v.as_str())
                            .unwrap_or("-"),
                        info.get("createdAt")
                            .and_then(|v| v.as_str())
                            .unwrap_or("-"),
                    );
                }
            }
        }
        OrdersAction::Get { order_id } => {
            let resp = client.get(&format!("/orders/{order_id}")).await?;
            let data: serde_json::Value = resp.json().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&data).unwrap());
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
                println!("{}", serde_json::to_string_pretty(&data).unwrap());
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
                    data.get("orderCost")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0)
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
                println!("{}", serde_json::to_string_pretty(&data).unwrap());
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
        OrdersAction::Download {
            order_id,
            deliverable_type,
        } => {
            let dtype = format!("{deliverable_type:?}").to_lowercase();
            let resp = client
                .get(&format!("/orders/{order_id}/{dtype}"))
                .await?;
            // This endpoint redirects to a download URL
            let url = resp.url().to_string();
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "download_url": url })
                );
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
                serde_json::from_str(&delivery_params).map_err(|e| {
                    CliError::General(format!("invalid delivery params JSON: {e}"))
                })?;
            let req = OrderRedeliveryRequest {
                delivery_driver,
                delivery_params: params,
            };
            let resp = client
                .post(&format!("/orders/{order_id}/redelivery"), &req)
                .await?;
            let data: serde_json::Value = resp.json().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&data).unwrap());
            } else {
                println!("Redelivery scheduled for order {order_id}");
            }
        }
    }
    Ok(())
}
