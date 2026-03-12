//! Notification-history-backed alert polling commands.
//!
//! These commands intentionally talk directly to the SkyFi Platform API rather
//! than the MCP server. They reuse the existing notifications endpoints:
//!
//! - `GET /notifications` to enumerate active monitors
//! - `GET /notifications/{id}` to fetch each monitor's history
//!
//! The CLI persists a local state file with seen event fingerprints so polling
//! remains incremental across invocations.

use crate::cli::{AlertsAction, AlertsStateAction};
use crate::client::Client;
use crate::error::CliError;
use crate::output;
use crate::types::{
    ListNotificationsResponse, NotificationResponse, NotificationWithHistoryResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::time::{sleep, Duration};

const MAX_SEEN_EVENT_KEYS: usize = 5000;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AlertsState {
    version: u32,
    #[serde(default)]
    seen_event_keys: Vec<String>,
    #[serde(default)]
    last_poll_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AlertRecord {
    notification_id: String,
    webhook_url: String,
    product_type: Option<String>,
    event_key: String,
    observed_at: Option<String>,
    event: Value,
}

#[derive(Debug, Serialize)]
struct PollResult {
    polled_at: String,
    total_notifications_checked: usize,
    new_count: usize,
    new_alerts: Vec<AlertRecord>,
}

pub async fn run(
    action: AlertsAction,
    client: &Client,
    json: bool,
    config_path: &Path,
) -> Result<(), CliError> {
    match action {
        AlertsAction::Poll { no_save_state } => {
            poll_once(client, json, config_path, !no_save_state).await
        }
        AlertsAction::Watch {
            interval,
            no_save_state,
        } => watch(client, json, config_path, interval, !no_save_state).await,
        AlertsAction::State { action } => state(action, json, config_path),
    }
}

async fn poll_once(
    client: &Client,
    json: bool,
    config_path: &Path,
    save_state: bool,
) -> Result<(), CliError> {
    let state_path = state_path(config_path);
    let mut state = AlertsState::load(&state_path)?;
    let result = fetch_unseen_alerts(client, &state).await?;

    if save_state {
        state.record_seen(&result.new_alerts, &result.polled_at);
        state.save(&state_path)?;
    }

    if json {
        output::print_json(&result)?;
    } else {
        print_human(&result, save_state);
    }

    Ok(())
}

async fn watch(
    client: &Client,
    json: bool,
    config_path: &Path,
    interval: u64,
    save_state: bool,
) -> Result<(), CliError> {
    loop {
        poll_once(client, json, config_path, save_state).await?;
        sleep(Duration::from_secs(interval)).await;
    }
}

fn state(action: AlertsStateAction, json: bool, config_path: &Path) -> Result<(), CliError> {
    let state_path = state_path(config_path);
    match action {
        AlertsStateAction::Show => {
            let state = AlertsState::load(&state_path)?;
            if json {
                output::print_json(&state)?;
            } else {
                println!("State file:  {}", state_path.display());
                println!("Seen events: {}", state.seen_event_keys.len());
                println!(
                    "Last poll:   {}",
                    state.last_poll_at.as_deref().unwrap_or("never")
                );
            }
        }
        AlertsStateAction::Reset => {
            let state = AlertsState::default();
            state.save(&state_path)?;
            if json {
                output::print_json(&serde_json::json!({
                    "status": "reset",
                    "state_file": state_path,
                }))?;
            } else {
                println!("Alerts state reset: {}", state_path.display());
            }
        }
    }
    Ok(())
}

async fn fetch_unseen_alerts(client: &Client, state: &AlertsState) -> Result<PollResult, CliError> {
    let resp = client.get("/notifications").await?;
    let notifications: ListNotificationsResponse = resp.json().await?;
    let polled_at = chrono::Utc::now().to_rfc3339();
    let mut new_alerts = Vec::new();
    let seen = state.seen_set();

    for notification in &notifications.notifications {
        let resp = client
            .get(&format!("/notifications/{}", notification.id))
            .await?;
        let detail: NotificationWithHistoryResponse = resp.json().await?;
        new_alerts.extend(collect_unseen_alerts(
            notification,
            detail.history.as_deref(),
            &seen,
        ));
    }

    Ok(PollResult {
        polled_at,
        total_notifications_checked: notifications.notifications.len(),
        new_count: new_alerts.len(),
        new_alerts,
    })
}

fn collect_unseen_alerts(
    notification: &NotificationResponse,
    history: Option<&[Value]>,
    seen: &HashSet<String>,
) -> Vec<AlertRecord> {
    history
        .unwrap_or(&[])
        .iter()
        .filter_map(|event| {
            let event_key = event_key(&notification.id, event);
            if seen.contains(&event_key) {
                return None;
            }

            Some(AlertRecord {
                notification_id: notification.id.clone(),
                webhook_url: notification.webhook_url.clone(),
                product_type: notification.product_type.as_ref().map(|v| format!("{v:?}")),
                observed_at: extract_observed_at(event),
                event_key,
                event: event.clone(),
            })
        })
        .collect()
}

fn print_human(result: &PollResult, save_state: bool) {
    eprintln!(
        "Checked {} notifications at {}.",
        result.total_notifications_checked, result.polled_at
    );

    if result.new_alerts.is_empty() {
        println!("No new alerts.");
        return;
    }

    for alert in &result.new_alerts {
        println!("Notification: {}", alert.notification_id);
        if let Some(product_type) = &alert.product_type {
            println!("Product type: {product_type}");
        }
        if let Some(observed_at) = &alert.observed_at {
            println!("Observed at:  {observed_at}");
        }
        println!("Webhook:      {}", alert.webhook_url);
        println!("Event:        {}", compact_json(&alert.event));
        println!();
    }

    println!(
        "Found {} new alert{}{}.",
        result.new_count,
        if result.new_count == 1 { "" } else { "s" },
        if save_state {
            " and saved them to local state"
        } else {
            ""
        }
    );
}

fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "<unserializable event>".to_string())
}

fn extract_observed_at(event: &Value) -> Option<String> {
    match event {
        Value::Object(map) => [
            "createdAt",
            "timestamp",
            "eventAt",
            "deliveredAt",
            "receivedAt",
            "created_at",
        ]
        .into_iter()
        .find_map(|key| map.get(key).and_then(Value::as_str).map(str::to_string)),
        _ => None,
    }
}

fn state_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("alerts-state.json")
}

fn event_key(notification_id: &str, event: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(notification_id.as_bytes());
    hasher.update(b":");
    hasher.update(canonical_json(event).as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        Value::String(v) => serde_json::to_string(v).unwrap_or_else(|_| "\"\"".to_string()),
        Value::Array(values) => {
            let items = values.iter().map(canonical_json).collect::<Vec<_>>();
            format!("[{}]", items.join(","))
        }
        Value::Object(values) => canonical_object(values),
    }
}

fn canonical_object(values: &Map<String, Value>) -> String {
    let mut entries = values.iter().collect::<Vec<_>>();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));

    let body = entries
        .into_iter()
        .map(|(key, value)| {
            format!(
                "{}:{}",
                serde_json::to_string(key).unwrap_or_else(|_| "\"\"".to_string()),
                canonical_json(value)
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    format!("{{{body}}}")
}

impl AlertsState {
    fn load(path: &Path) -> Result<Self, CliError> {
        if !path.exists() {
            return Ok(Self {
                version: 1,
                ..Self::default()
            });
        }

        let content = fs::read_to_string(path)?;
        let mut state: Self = serde_json::from_str(&content)?;
        if state.version == 0 {
            state.version = 1;
        }
        Ok(state)
    }

    fn save(&self, path: &Path) -> Result<(), CliError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    fn seen_set(&self) -> HashSet<String> {
        self.seen_event_keys.iter().cloned().collect()
    }

    fn record_seen(&mut self, alerts: &[AlertRecord], polled_at: &str) {
        let mut combined = self.seen_event_keys.iter().cloned().collect::<Vec<_>>();
        let mut seen = combined.iter().cloned().collect::<HashSet<_>>();

        for alert in alerts {
            if seen.insert(alert.event_key.clone()) {
                combined.push(alert.event_key.clone());
            }
        }

        if combined.len() > MAX_SEEN_EVENT_KEYS {
            let drop_count = combined.len() - MAX_SEEN_EVENT_KEYS;
            combined.drain(0..drop_count);
        }

        self.version = 1;
        self.seen_event_keys = combined;
        self.last_poll_at = Some(polled_at.to_string());
    }
}

#[cfg(test)]
#[path = "alerts_test.rs"]
mod tests;
