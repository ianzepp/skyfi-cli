use crate::cli::NotificationsAction;
use crate::client::Client;
use crate::error::CliError;
use crate::output;
use crate::types::*;

pub async fn run(action: NotificationsAction, client: &Client, json: bool) -> Result<(), CliError> {
    match action {
        NotificationsAction::List { page, page_size } => {
            let mut query: Vec<(&str, String)> = vec![];
            if let Some(p) = page {
                query.push(("pageNumber", p.to_string()));
            }
            if let Some(ps) = page_size {
                query.push(("pageSize", ps.to_string()));
            }
            let resp = client.get_query("/notifications", &query).await?;
            let data: ListNotificationsResponse = resp.json().await?;
            if json {
                output::print_json(&data)?;
            } else {
                eprintln!("Total: {}", data.total);
                for n in &data.notifications {
                    println!(
                        "{:<36}  {:?}  {}",
                        n.id,
                        n.product_type
                            .as_ref()
                            .map(|pt| format!("{pt:?}"))
                            .unwrap_or_default(),
                        n.created_at,
                    );
                }
            }
        }
        NotificationsAction::Get { notification_id } => {
            let resp = client
                .get(&format!("/notifications/{notification_id}"))
                .await?;
            let data: NotificationWithHistoryResponse = resp.json().await?;
            if json {
                output::print_json(&data)?;
            } else {
                let n = &data.notification;
                println!("ID:           {}", n.id);
                println!("Webhook:      {}", n.webhook_url);
                println!("Created:      {}", n.created_at);
                if let Some(pt) = &n.product_type {
                    println!("Product type: {pt:?}");
                }
                if let Some(min) = n.gsd_min {
                    println!("GSD min:      {min}");
                }
                if let Some(max) = n.gsd_max {
                    println!("GSD max:      {max}");
                }
                if let Some(history) = &data.history {
                    println!("\nHistory ({} events):", history.len());
                    for event in history {
                        println!("  {event}");
                    }
                }
            }
        }
        NotificationsAction::Create {
            aoi,
            webhook_url,
            gsd_min,
            gsd_max,
            product_type,
        } => {
            let req = CreateNotificationRequest {
                aoi,
                webhook_url,
                gsd_min,
                gsd_max,
                product_type,
            };
            let resp = client.post("/notifications", &req).await?;
            let data: NotificationResponse = resp.json().await?;
            if json {
                output::print_json(&data)?;
            } else {
                println!("Notification created: {}", data.id);
            }
        }
        NotificationsAction::Delete { notification_id } => {
            let resp = client
                .delete(&format!("/notifications/{notification_id}"))
                .await?;
            let data: StatusResponse = resp.json().await?;
            if json {
                output::print_json(&data)?;
            } else {
                println!("Deleted: {}", data.status);
            }
        }
    }
    Ok(())
}
