use super::{
    canonical_json, collect_unseen_alerts, event_key, extract_observed_at,
    render_launch_agent_plist, render_systemd_service, render_systemd_timer, shell_quote,
    state_path, xml_escape, AlertRecord, AlertsState,
};
use crate::types::{NotificationResponse, ProductType};
use serde_json::json;
use std::path::Path;

fn notification() -> NotificationResponse {
    NotificationResponse {
        id: "notif-1".to_string(),
        owner_id: "owner-1".to_string(),
        aoi: "POLYGON((0 0,1 0,1 1,0 0))".to_string(),
        gsd_min: None,
        gsd_max: Some(5),
        product_type: Some(ProductType::Day),
        webhook_url: "https://example.com/hook".to_string(),
        created_at: "2026-03-12T00:00:00Z".to_string(),
    }
}

#[test]
fn canonical_json_sorts_object_keys_recursively() {
    let left = json!({
        "b": 1,
        "a": {
            "d": 4,
            "c": 3
        }
    });
    let right = json!({
        "a": {
            "c": 3,
            "d": 4
        },
        "b": 1
    });

    assert_eq!(canonical_json(&left), canonical_json(&right));
}

#[test]
fn event_key_is_stable_for_reordered_objects() {
    let left = json!({"b": 1, "a": 2});
    let right = json!({"a": 2, "b": 1});

    assert_eq!(event_key("notif-1", &left), event_key("notif-1", &right));
}

#[test]
fn collect_unseen_alerts_filters_previously_seen_events() {
    let notification = notification();
    let first = json!({"createdAt": "2026-03-12T00:00:00Z", "archiveId": "arch-1"});
    let second = json!({"createdAt": "2026-03-13T00:00:00Z", "archiveId": "arch-2"});
    let seen = [event_key(&notification.id, &first)].into_iter().collect();

    let alerts = collect_unseen_alerts(&notification, Some(&[first, second.clone()]), &seen);

    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].notification_id, "notif-1");
    assert_eq!(
        alerts[0].observed_at.as_deref(),
        Some("2026-03-13T00:00:00Z")
    );
    assert_eq!(alerts[0].event, second);
}

#[test]
fn extract_observed_at_prefers_known_timestamp_fields() {
    let event = json!({"timestamp": "2026-03-14T10:00:00Z"});
    assert_eq!(
        extract_observed_at(&event).as_deref(),
        Some("2026-03-14T10:00:00Z")
    );
}

#[test]
fn alerts_state_record_seen_deduplicates_and_trims() {
    let mut state = AlertsState {
        version: 1,
        seen_event_keys: (0..5000).map(|i| format!("existing-{i}")).collect(),
        last_poll_at: None,
    };

    let alerts = vec![
        AlertRecord {
            notification_id: "notif-1".to_string(),
            webhook_url: "https://example.com/hook".to_string(),
            product_type: Some("Day".to_string()),
            event_key: "existing-4999".to_string(),
            observed_at: None,
            event: json!({"id": 1}),
        },
        AlertRecord {
            notification_id: "notif-1".to_string(),
            webhook_url: "https://example.com/hook".to_string(),
            product_type: Some("Day".to_string()),
            event_key: "new-1".to_string(),
            observed_at: None,
            event: json!({"id": 2}),
        },
    ];

    state.record_seen(&alerts, "2026-03-12T00:00:00Z");

    assert_eq!(state.seen_event_keys.len(), 5000);
    assert!(state.seen_event_keys.contains(&"new-1".to_string()));
    assert!(!state.seen_event_keys.contains(&"existing-0".to_string()));
    assert_eq!(state.last_poll_at.as_deref(), Some("2026-03-12T00:00:00Z"));
}

#[test]
fn state_path_uses_config_directory_sibling_file() {
    let path = state_path(Path::new("/tmp/skyfi/config.toml"));
    assert_eq!(path, Path::new("/tmp/skyfi/alerts-state.json"));
}

#[test]
fn xml_escape_escapes_plist_special_characters() {
    assert_eq!(
        xml_escape("A&B\"<'test'>"),
        "A&amp;B&quot;&lt;&apos;test&apos;&gt;"
    );
}

#[test]
fn render_launch_agent_plist_includes_service_run_arguments() {
    let plist = render_launch_agent_plist(
        "com.skyfi.alerts",
        Path::new("/usr/local/bin/skyfi"),
        Path::new("/tmp/skyfi/config.toml"),
        300,
        Some(Path::new("/tmp/hook.sh")),
        Path::new("/tmp/skyfi/stdout.log"),
        Path::new("/tmp/skyfi/stderr.log"),
    );

    assert!(plist.contains("<string>/usr/local/bin/skyfi</string>"));
    assert!(plist.contains("<string>--config</string>"));
    assert!(plist.contains("<string>/tmp/skyfi/config.toml</string>"));
    assert!(plist.contains("<string>alerts</string>"));
    assert!(plist.contains("<string>service-run</string>"));
    assert!(plist.contains("<string>--on-alert</string>"));
    assert!(plist.contains("<string>/tmp/hook.sh</string>"));
    assert!(plist.contains("<integer>300</integer>"));
}

#[test]
fn render_systemd_service_includes_service_run_arguments() {
    let unit = render_systemd_service(
        Path::new("/usr/local/bin/skyfi"),
        Path::new("/tmp/skyfi/config.toml"),
        Some(Path::new("/tmp/hook.sh")),
    );

    assert!(unit.contains("Description=SkyFi alert polling service"));
    assert!(unit.contains("ExecStart=/usr/local/bin/skyfi --config /tmp/skyfi/config.toml alerts service-run --on-alert /tmp/hook.sh"));
}

#[test]
fn render_systemd_timer_uses_requested_interval() {
    let timer = render_systemd_timer(300);
    assert!(timer.contains("OnBootSec=30s"));
    assert!(timer.contains("OnUnitActiveSec=300s"));
    assert!(timer.contains("Unit=skyfi-alerts.service"));
}

#[test]
fn shell_quote_quotes_paths_with_spaces() {
    assert_eq!(
        shell_quote("/tmp/skyfi alert hook.sh"),
        "'/tmp/skyfi alert hook.sh'"
    );
}
