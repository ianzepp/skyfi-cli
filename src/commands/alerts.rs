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
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tokio::time::{sleep, Duration};

const MAX_SEEN_EVENT_KEYS: usize = 5000;
const LAUNCH_AGENT_LABEL: &str = "com.skyfi.alerts";
const SYSTEMD_SERVICE_NAME: &str = "skyfi-alerts.service";
const SYSTEMD_TIMER_NAME: &str = "skyfi-alerts.timer";

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
        AlertsAction::Install {
            interval,
            on_alert,
            no_load,
        } => install(json, config_path, interval, on_alert.as_deref(), !no_load),
        AlertsAction::ServiceRun { on_alert } => service_run(client, config_path, on_alert).await,
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

fn install(
    json: bool,
    config_path: &Path,
    interval: u64,
    on_alert: Option<&Path>,
    load_now: bool,
) -> Result<(), CliError> {
    let binary = env::current_exe()?;
    if cfg!(target_os = "macos") {
        install_macos(json, config_path, interval, on_alert, load_now, &binary)
    } else if cfg!(target_os = "linux") {
        install_linux(json, config_path, interval, on_alert, load_now, &binary)
    } else {
        Err(CliError::General(
            "alerts install currently supports macOS launchd and Linux systemd --user only"
                .to_string(),
        ))
    }
}

async fn service_run(
    client: &Client,
    config_path: &Path,
    on_alert: Option<PathBuf>,
) -> Result<(), CliError> {
    let state_path = state_path(config_path);
    let mut state = AlertsState::load(&state_path)?;
    let result = fetch_unseen_alerts(client, &state).await?;

    if result.new_alerts.is_empty() {
        return Ok(());
    }

    state.record_seen(&result.new_alerts, &result.polled_at);
    state.save(&state_path)?;

    for alert in &result.new_alerts {
        notify_local(alert)?;
        if let Some(path) = on_alert.as_deref() {
            run_on_alert_hook(path, alert)?;
        }
    }

    Ok(())
}

fn install_macos(
    json: bool,
    config_path: &Path,
    interval: u64,
    on_alert: Option<&Path>,
    load_now: bool,
    binary: &Path,
) -> Result<(), CliError> {
    let launch_agents_dir = dirs::home_dir()
        .ok_or_else(|| CliError::General("could not determine home directory".to_string()))?
        .join("Library")
        .join("LaunchAgents");
    fs::create_dir_all(&launch_agents_dir)?;

    let agent_path = launch_agents_dir.join(format!("{LAUNCH_AGENT_LABEL}.plist"));
    let logs_dir = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("logs");
    fs::create_dir_all(&logs_dir)?;
    let stdout_path = logs_dir.join("alerts-service.log");
    let stderr_path = logs_dir.join("alerts-service.error.log");
    let plist = render_launch_agent_plist(
        LAUNCH_AGENT_LABEL,
        binary,
        config_path,
        interval,
        on_alert,
        &stdout_path,
        &stderr_path,
    );
    fs::write(&agent_path, plist)?;

    if load_now {
        let domain = launchctl_domain()?;
        let _ = Command::new("launchctl")
            .args(["bootout", &domain, &agent_path.display().to_string()])
            .status();

        let status = Command::new("launchctl")
            .args(["bootstrap", &domain, &agent_path.display().to_string()])
            .status()?;
        if !status.success() {
            return Err(CliError::General(format!(
                "failed to load launch agent {}",
                agent_path.display()
            )));
        }
    }

    let payload = serde_json::json!({
        "platform": "macos",
        "label": LAUNCH_AGENT_LABEL,
        "plist_path": agent_path,
        "interval": interval,
        "binary": binary,
        "config_path": config_path,
        "on_alert": on_alert,
        "loaded": load_now,
        "stdout_log": stdout_path,
        "stderr_log": stderr_path,
    });

    if json {
        output::print_json(&payload)?;
    } else {
        println!("Installed launch agent: {}", agent_path.display());
        println!("Label:                {LAUNCH_AGENT_LABEL}");
        println!("Interval:             {interval}s");
        println!("Config:               {}", config_path.display());
        println!("Binary:               {}", binary.display());
        if let Some(path) = on_alert {
            println!("On-alert hook:        {}", path.display());
        }
        println!("Stdout log:           {}", stdout_path.display());
        println!("Stderr log:           {}", stderr_path.display());
        println!(
            "{}",
            if load_now {
                "The service has been loaded with launchd."
            } else {
                "The plist was written but not loaded (--no-load)."
            }
        );
    }

    Ok(())
}

fn install_linux(
    json: bool,
    config_path: &Path,
    interval: u64,
    on_alert: Option<&Path>,
    load_now: bool,
    binary: &Path,
) -> Result<(), CliError> {
    let units_dir = dirs::config_dir()
        .ok_or_else(|| CliError::General("could not determine config directory".to_string()))?
        .join("systemd")
        .join("user");
    fs::create_dir_all(&units_dir)?;

    let service_path = units_dir.join(SYSTEMD_SERVICE_NAME);
    let timer_path = units_dir.join(SYSTEMD_TIMER_NAME);
    fs::write(
        &service_path,
        render_systemd_service(binary, config_path, on_alert),
    )?;
    fs::write(&timer_path, render_systemd_timer(interval))?;

    if load_now {
        let daemon_reload = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status()?;
        if !daemon_reload.success() {
            return Err(CliError::General(
                "failed to reload systemd user units".to_string(),
            ));
        }

        let enable_timer = Command::new("systemctl")
            .args(["--user", "enable", "--now", SYSTEMD_TIMER_NAME])
            .status()?;
        if !enable_timer.success() {
            return Err(CliError::General(format!(
                "failed to enable/start {}",
                SYSTEMD_TIMER_NAME
            )));
        }
    }

    let payload = serde_json::json!({
        "platform": "linux",
        "service_name": SYSTEMD_SERVICE_NAME,
        "timer_name": SYSTEMD_TIMER_NAME,
        "service_path": service_path,
        "timer_path": timer_path,
        "interval": interval,
        "binary": binary,
        "config_path": config_path,
        "on_alert": on_alert,
        "loaded": load_now,
    });

    if json {
        output::print_json(&payload)?;
    } else {
        println!("Installed systemd user service: {}", service_path.display());
        println!("Installed systemd user timer:   {}", timer_path.display());
        println!("Service:                        {SYSTEMD_SERVICE_NAME}");
        println!("Timer:                          {SYSTEMD_TIMER_NAME}");
        println!("Interval:                       {interval}s");
        println!("Config:                         {}", config_path.display());
        println!("Binary:                         {}", binary.display());
        if let Some(path) = on_alert {
            println!("On-alert hook:                  {}", path.display());
        }
        println!(
            "{}",
            if load_now {
                "The timer has been enabled and started with systemd --user."
            } else {
                "The unit files were written but not enabled (--no-load)."
            }
        );
    }

    Ok(())
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

fn render_launch_agent_plist(
    label: &str,
    binary: &Path,
    config_path: &Path,
    interval: u64,
    on_alert: Option<&Path>,
    stdout_path: &Path,
    stderr_path: &Path,
) -> String {
    let mut args = vec![
        binary.display().to_string(),
        "--config".to_string(),
        config_path.display().to_string(),
        "alerts".to_string(),
        "service-run".to_string(),
    ];
    if let Some(path) = on_alert {
        args.push("--on-alert".to_string());
        args.push(path.display().to_string());
    }
    let arguments = args
        .into_iter()
        .map(|arg| format!("    <string>{}</string>", xml_escape(&arg)))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
{arguments}
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>StartInterval</key>
  <integer>{interval}</integer>
  <key>StandardOutPath</key>
  <string>{stdout_path}</string>
  <key>StandardErrorPath</key>
  <string>{stderr_path}</string>
</dict>
</plist>
"#,
        label = xml_escape(label),
        arguments = arguments,
        interval = interval,
        stdout_path = xml_escape(&stdout_path.display().to_string()),
        stderr_path = xml_escape(&stderr_path.display().to_string()),
    )
}

fn render_systemd_service(binary: &Path, config_path: &Path, on_alert: Option<&Path>) -> String {
    let exec_start = shell_join_command(binary, config_path, on_alert);
    format!(
        "[Unit]\nDescription=SkyFi alert polling service\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=oneshot\nExecStart={exec_start}\n"
    )
}

fn render_systemd_timer(interval: u64) -> String {
    format!(
        "[Unit]\nDescription=Run SkyFi alert polling periodically\n\n[Timer]\nOnBootSec=30s\nOnUnitActiveSec={interval}s\nPersistent=true\nUnit={SYSTEMD_SERVICE_NAME}\n\n[Install]\nWantedBy=timers.target\n"
    )
}

fn shell_join_command(binary: &Path, config_path: &Path, on_alert: Option<&Path>) -> String {
    let mut args = vec![
        binary.display().to_string(),
        "--config".to_string(),
        config_path.display().to_string(),
        "alerts".to_string(),
        "service-run".to_string(),
    ];
    if let Some(path) = on_alert {
        args.push("--on-alert".to_string());
        args.push(path.display().to_string());
    }

    args.into_iter()
        .map(|arg| shell_quote(&arg))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    let safe = value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "/._-:@".contains(c));
    if safe {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    }
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn launchctl_domain() -> Result<String, CliError> {
    if let Ok(uid) = env::var("UID") {
        let trimmed = uid.trim();
        if !trimmed.is_empty() {
            return Ok(format!("gui/{trimmed}"));
        }
    }

    let output = Command::new("id").arg("-u").output()?;
    if !output.status.success() {
        return Err(CliError::General(
            "failed to determine current uid for launchctl".to_string(),
        ));
    }

    let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if uid.is_empty() {
        return Err(CliError::General(
            "failed to determine current uid for launchctl".to_string(),
        ));
    }

    Ok(format!("gui/{uid}"))
}

fn notify_local(alert: &AlertRecord) -> Result<(), CliError> {
    if cfg!(target_os = "macos") {
        return notify_macos(alert);
    }
    if cfg!(target_os = "linux") {
        return notify_linux(alert);
    }
    Ok(())
}

fn notify_macos(alert: &AlertRecord) -> Result<(), CliError> {
    let mut parts = vec![format!("New imagery alert for {}", alert.notification_id)];
    if let Some(product_type) = &alert.product_type {
        parts.push(format!("Product: {product_type}"));
    }
    if let Some(observed_at) = &alert.observed_at {
        parts.push(format!("Observed: {observed_at}"));
    }
    let body = parts.join(" | ");

    let status = Command::new("osascript")
        .args([
            "-e",
            "on run argv",
            "-e",
            "display notification (item 1 of argv) with title (item 2 of argv)",
            "-e",
            "end run",
            &body,
            "SkyFi Alert",
        ])
        .status()?;
    if !status.success() {
        return Err(CliError::General(
            "failed to post macOS notification via osascript".to_string(),
        ));
    }
    Ok(())
}

fn notify_linux(alert: &AlertRecord) -> Result<(), CliError> {
    let body = build_notification_body(alert);
    let status = Command::new("notify-send")
        .args(["SkyFi Alert", &body])
        .status();

    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn build_notification_body(alert: &AlertRecord) -> String {
    let mut parts = vec![format!("New imagery alert for {}", alert.notification_id)];
    if let Some(product_type) = &alert.product_type {
        parts.push(format!("Product: {product_type}"));
    }
    if let Some(observed_at) = &alert.observed_at {
        parts.push(format!("Observed: {observed_at}"));
    }
    parts.join(" | ")
}

fn run_on_alert_hook(path: &Path, alert: &AlertRecord) -> Result<(), CliError> {
    let payload = serde_json::to_vec(alert)?;
    let mut command = Command::new(path);
    command
        .env("SKYFI_ALERT_NOTIFICATION_ID", &alert.notification_id)
        .env("SKYFI_ALERT_WEBHOOK_URL", &alert.webhook_url)
        .env("SKYFI_ALERT_EVENT_KEY", &alert.event_key)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Some(product_type) = &alert.product_type {
        command.env("SKYFI_ALERT_PRODUCT_TYPE", product_type);
    }
    if let Some(observed_at) = &alert.observed_at {
        command.env("SKYFI_ALERT_OBSERVED_AT", observed_at);
    }

    let mut child = command.spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(&payload)?;
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(CliError::General(format!(
            "alert hook failed: {}",
            path.display()
        )));
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
