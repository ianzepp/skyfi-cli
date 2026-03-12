use crate::types::{
    ApiProvider, ArchiveOrderRequest, CreateNotificationRequest, FeasibilityRequest,
    FeasibilityTaskResponse, GetArchivesRequest, GetArchivesResponse, ListNotificationsResponse,
    ListOrdersResponse, NotificationWithHistoryResponse, OrderRedeliveryRequest,
    PassPredictionRequest, PassPredictionResponse, PongResponse, PricingRequest, ProductType,
    TaskingOrderRequest, WhoamiUser,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

struct OpenApiContract {
    document: Value,
}

impl OpenApiContract {
    fn load() -> Self {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("openapi.json");
        let content =
            fs::read_to_string(path).expect("openapi.json should be readable for contract tests");
        let document = serde_json::from_str(&content).expect("openapi.json should be valid JSON");
        Self { document }
    }

    fn schema(&self, name: &str) -> &Value {
        &self.document["components"]["schemas"][name]
    }

    fn property_names(&self, name: &str) -> BTreeSet<String> {
        self.schema(name)["properties"]
            .as_object()
            .expect("schema should define properties")
            .keys()
            .cloned()
            .collect()
    }

    fn required_names(&self, name: &str) -> BTreeSet<String> {
        self.schema(name)["required"]
            .as_array()
            .map(|required| {
                required
                    .iter()
                    .filter_map(|entry| entry.as_str().map(ToOwned::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    }
}

struct MockSkyfiServer;

impl MockSkyfiServer {
    fn pong_response() -> Value {
        json!({ "message": "pong" })
    }

    fn whoami_response() -> Value {
        json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "organizationId": "550e8400-e29b-41d4-a716-446655440001",
            "email": "ops@example.com",
            "firstName": "Sky",
            "lastName": "Fi",
            "isDemoAccount": false,
            "currentBudgetUsage": 1200,
            "budgetAmount": 5000,
            "hasValidSharedCard": true
        })
    }

    fn archives_response() -> Value {
        json!({
            "request": {
                "aoi": "POLYGON((-99.919 16.847,-99.921 16.826,-99.899 16.825,-99.899 16.849,-99.919 16.847))"
            },
            "archives": [
                {
                    "archiveId": "archive-123",
                    "provider": "PLANET",
                    "constellation": "SKYSAT",
                    "productType": "DAY",
                    "platformResolution": 50.0,
                    "resolution": "HIGH",
                    "captureTimestamp": "2025-01-15T00:00:00Z",
                    "cloudCoveragePercent": 12.5,
                    "offNadirAngle": 4.5,
                    "footprint": "POLYGON((-99.919 16.847,-99.921 16.826,-99.899 16.825,-99.899 16.849,-99.919 16.847))",
                    "minSqKm": 5.0,
                    "maxSqKm": 50.0,
                    "priceForOneSquareKm": 12.5,
                    "priceForOneSquareKmCents": 1250,
                    "priceFullScene": 250.0,
                    "openData": false,
                    "totalAreaSquareKm": 20.0,
                    "deliveryTimeHours": 12.0,
                    "thumbnailUrls": {
                        "200x200": "https://example.com/thumb.png"
                    },
                    "gsd": 0.5,
                    "tilesUrl": "https://example.com/tiles/{z}/{x}/{y}",
                    "overlapRatio": 0.9,
                    "overlapSqkm": 18.0
                }
            ],
            "nextPage": null,
            "total": 1
        })
    }

    fn list_orders_response() -> Value {
        json!({
            "request": {
                "orderType": "ARCHIVE",
                "pageNumber": 0,
                "pageSize": 25,
                "sortColumns": ["created_at"],
                "sortDirections": ["desc"]
            },
            "total": 1,
            "orders": [
                {
                    "aoi": "POLYGON((-99.919 16.847,-99.921 16.826,-99.899 16.825,-99.899 16.849,-99.919 16.847))",
                    "archiveId": "archive-123",
                    "id": "550e8400-e29b-41d4-a716-446655440010",
                    "orderType": "ARCHIVE",
                    "orderCost": 1200,
                    "ownerId": "550e8400-e29b-41d4-a716-446655440011",
                    "status": "DELIVERY_COMPLETED",
                    "aoiSqkm": 20.0,
                    "downloadImageUrl": "https://example.com/image.tif",
                    "downloadPayloadUrl": "https://example.com/payload.zip",
                    "downloadCogUrl": "https://example.com/image.cog.tif",
                    "orderCode": "ORD-123",
                    "createdAt": "2025-01-15T00:00:00Z",
                    "orderId": "550e8400-e29b-41d4-a716-446655440012",
                    "itemId": "550e8400-e29b-41d4-a716-446655440013",
                    "archive": {
                        "archiveId": "archive-123",
                        "provider": "PLANET",
                        "constellation": "SKYSAT",
                        "productType": "DAY",
                        "platformResolution": 50.0,
                        "resolution": "HIGH",
                        "captureTimestamp": "2025-01-15T00:00:00Z",
                        "footprint": "POLYGON((-99.919 16.847,-99.921 16.826,-99.899 16.825,-99.899 16.849,-99.919 16.847))",
                        "minSqKm": 5.0,
                        "maxSqKm": 50.0,
                        "priceForOneSquareKm": 12.5,
                        "priceForOneSquareKmCents": 1250,
                        "priceFullScene": 250.0,
                        "totalAreaSquareKm": 20.0,
                        "gsd": 0.5
                    }
                }
            ]
        })
    }

    fn list_notifications_response() -> Value {
        json!({
            "request": {
                "pageNumber": 0,
                "pageSize": 25
            },
            "total": 1,
            "notifications": [
                {
                    "id": "550e8400-e29b-41d4-a716-446655440020",
                    "ownerId": "550e8400-e29b-41d4-a716-446655440021",
                    "aoi": "POLYGON((-73.81 40.47,-73.83 40.41,-73.73 40.43,-73.81 40.47))",
                    "gsdMin": 1,
                    "gsdMax": 5,
                    "productType": "DAY",
                    "webhookUrl": "https://example.com/hook",
                    "createdAt": "2025-01-15T00:00:00Z"
                }
            ]
        })
    }

    fn notification_with_history_response() -> Value {
        json!({
            "id": "550e8400-e29b-41d4-a716-446655440020",
            "ownerId": "550e8400-e29b-41d4-a716-446655440021",
            "aoi": "POLYGON((-73.81 40.47,-73.83 40.41,-73.73 40.43,-73.81 40.47))",
            "gsdMin": 1,
            "gsdMax": 5,
            "productType": "DAY",
            "webhookUrl": "https://example.com/hook",
            "createdAt": "2025-01-15T00:00:00Z",
            "history": [
                {}
            ]
        })
    }

    fn feasibility_task_response() -> Value {
        json!({
            "id": "550e8400-e29b-41d4-a716-446655440030",
            "validUntil": "2025-01-16T00:00:00Z",
            "overallScore": {
                "feasibility": 0.82,
                "providerScore": {
                    "providerScores": []
                }
            }
        })
    }

    fn pass_prediction_response() -> Value {
        json!({
            "passes": [
                {
                    "provider": "PLANET",
                    "satname": "SKYSAT-101",
                    "satid": "SKYSAT-101",
                    "noradid": "2024-0101",
                    "node": "ascending",
                    "productType": "DAY",
                    "resolution": "HIGH",
                    "lat": 30.0,
                    "lon": -97.0,
                    "passDate": "2025-01-15T00:00:00Z",
                    "meanT": 18,
                    "offNadirAngle": 12.5,
                    "solarElevationAngle": 35.0,
                    "minSquareKms": 10.0,
                    "maxSquareKms": 50.0,
                    "priceForOneSquareKm": 12.5,
                    "priceForOneSquareKmCents": 1250,
                    "gsdDegMin": 0.4,
                    "gsdDegMax": 0.8
                }
            ]
        })
    }
}

fn assert_request_matches_schema(schema_name: &str, payload: &Value, contract: &OpenApiContract) {
    let object = payload
        .as_object()
        .expect("request payload should serialize to a JSON object");
    let keys: BTreeSet<String> = object.keys().cloned().collect();
    let property_names = contract.property_names(schema_name);
    let required_names = contract.required_names(schema_name);

    for key in &keys {
        assert!(
            property_names.contains(key),
            "schema {schema_name} does not define property {key}"
        );
    }

    for required in &required_names {
        assert!(
            keys.contains(required),
            "schema {schema_name} requires property {required}"
        );
    }
}

#[test]
fn request_types_serialize_with_openapi_property_names() {
    let contract = OpenApiContract::load();

    let get_archives = serde_json::to_value(GetArchivesRequest {
        aoi:
            "POLYGON((-99.919 16.847,-99.921 16.826,-99.899 16.825,-99.899 16.849,-99.919 16.847))"
                .to_string(),
        from_date: Some("2025-01-01T00:00:00Z".to_string()),
        to_date: Some("2025-01-31T00:00:00Z".to_string()),
        max_cloud_coverage_percent: Some(25.0),
        max_off_nadir_angle: Some(15.0),
        resolutions: Some(vec!["HIGH".to_string()]),
        product_types: Some(vec![ProductType::Day]),
        providers: Some(vec![ApiProvider::Planet]),
        open_data: Some(false),
        min_overlap_ratio: Some(0.8),
        page_number: Some(0),
        page_size: 25,
    })
    .expect("request should serialize");
    assert_request_matches_schema("GetArchivesRequest", &get_archives, &contract);

    let archive_order = serde_json::to_value(ArchiveOrderRequest {
        aoi:
            "POLYGON((-99.919 16.847,-99.921 16.826,-99.899 16.825,-99.899 16.849,-99.919 16.847))"
                .to_string(),
        archive_id: "archive-123".to_string(),
        label: Some("Archive order".to_string()),
        order_label: Some("Archive order".to_string()),
        delivery_driver: None,
        delivery_params: None,
        webhook_url: Some("https://example.com/hook".to_string()),
        metadata: None,
    })
    .expect("request should serialize");
    assert_request_matches_schema("ArchiveOrderRequest", &archive_order, &contract);

    let tasking_order = serde_json::to_value(TaskingOrderRequest {
        aoi:
            "POLYGON((-99.919 16.847,-99.921 16.826,-99.899 16.825,-99.899 16.849,-99.919 16.847))"
                .to_string(),
        window_start: "2025-01-15T00:00:00Z".to_string(),
        window_end: "2025-01-20T00:00:00Z".to_string(),
        product_type: ProductType::Day,
        resolution: "HIGH".to_string(),
        label: Some("Tasking order".to_string()),
        order_label: Some("Tasking order".to_string()),
        priority_item: Some(true),
        max_cloud_coverage_percent: Some(10),
        max_off_nadir_angle: Some(15),
        required_provider: Some(ApiProvider::Planet),
        delivery_driver: None,
        delivery_params: None,
        webhook_url: Some("https://example.com/hook".to_string()),
        metadata: None,
        sar_product_types: None,
        sar_polarisation: None,
        provider_window_id: Some("550e8400-e29b-41d4-a716-446655440099".to_string()),
    })
    .expect("request should serialize");
    assert_request_matches_schema("TaskingOrderRequest", &tasking_order, &contract);

    let notification = serde_json::to_value(CreateNotificationRequest {
        aoi: "POLYGON((-73.81 40.47,-73.83 40.41,-73.73 40.43,-73.81 40.47))".to_string(),
        webhook_url: "https://example.com/hook".to_string(),
        gsd_min: Some(1),
        gsd_max: Some(5),
        product_type: Some(ProductType::Day),
    })
    .expect("request should serialize");
    assert_request_matches_schema("CreateNotificationRequest", &notification, &contract);

    let pricing = serde_json::to_value(PricingRequest {
        aoi: Some("POLYGON((-73.81 40.47,-73.83 40.41,-73.73 40.43,-73.81 40.47))".to_string()),
    })
    .expect("request should serialize");
    assert_request_matches_schema("PricingRequest", &pricing, &contract);

    let feasibility = serde_json::to_value(FeasibilityRequest {
        aoi:
            "POLYGON((-99.919 16.847,-99.921 16.826,-99.899 16.825,-99.899 16.849,-99.919 16.847))"
                .to_string(),
        product_type: ProductType::Day,
        resolution: "HIGH".to_string(),
        start_date: "2025-01-15T00:00:00Z".to_string(),
        end_date: "2025-01-20T00:00:00Z".to_string(),
        max_cloud_coverage_percent: Some(20.0),
        priority_item: Some(false),
        required_provider: Some("PLANET".to_string()),
    })
    .expect("request should serialize");
    assert_request_matches_schema("PlatformApiFeasibilityTaskRequest", &feasibility, &contract);
    assert!(
        feasibility["aoi"].is_string(),
        "feasibility aoi must be a WKT string"
    );

    let pass_prediction = serde_json::to_value(PassPredictionRequest {
        aoi:
            "POLYGON((-99.919 16.847,-99.921 16.826,-99.899 16.825,-99.899 16.849,-99.919 16.847))"
                .to_string(),
        from_date: "2025-01-15T00:00:00Z".to_string(),
        to_date: "2025-01-20T00:00:00Z".to_string(),
        product_types: Some(vec![ProductType::Day]),
        resolutions: Some(vec!["HIGH".to_string()]),
        max_off_nadir_angle: Some(20.0),
    })
    .expect("request should serialize");
    assert_request_matches_schema(
        "PlatformApiPassPredictionRequest",
        &pass_prediction,
        &contract,
    );
    assert!(
        pass_prediction["aoi"].is_string(),
        "pass prediction aoi must be a WKT string"
    );

    let redelivery = serde_json::to_value(OrderRedeliveryRequest {
        delivery_driver: crate::types::DeliveryDriver::S3,
        delivery_params: std::collections::HashMap::from([(
            "s3_bucket_id".to_string(),
            json!("imagery-bucket"),
        )]),
    })
    .expect("request should serialize");
    assert_request_matches_schema("OrderRedeliveryRequest", &redelivery, &contract);
}

#[test]
fn official_mock_responses_deserialize_into_cli_types() {
    let pong: PongResponse = serde_json::from_value(MockSkyfiServer::pong_response())
        .expect("pong response should deserialize");
    assert_eq!(pong.message, "pong");

    let whoami: WhoamiUser = serde_json::from_value(MockSkyfiServer::whoami_response())
        .expect("whoami response should deserialize");
    assert_eq!(whoami.email, "ops@example.com");

    let archives: GetArchivesResponse =
        serde_json::from_value(MockSkyfiServer::archives_response())
            .expect("archives response should deserialize");
    assert_eq!(archives.archives.len(), 1);
    assert_eq!(
        archives.archives[0].archive.price_for_one_square_km_cents,
        1250
    );

    let orders: ListOrdersResponse =
        serde_json::from_value(MockSkyfiServer::list_orders_response())
            .expect("orders response should deserialize");
    assert_eq!(orders.total, 1);
    assert_eq!(orders.orders[0].order_code, "ORD-123");

    let notifications: ListNotificationsResponse =
        serde_json::from_value(MockSkyfiServer::list_notifications_response())
            .expect("notifications response should deserialize");
    assert_eq!(notifications.notifications.len(), 1);

    let notification: NotificationWithHistoryResponse =
        serde_json::from_value(MockSkyfiServer::notification_with_history_response())
            .expect("notification history response should deserialize");
    assert_eq!(
        notification.notification.id,
        "550e8400-e29b-41d4-a716-446655440020"
    );
    assert_eq!(notification.history.as_ref().map(Vec::len), Some(1));

    let feasibility: FeasibilityTaskResponse =
        serde_json::from_value(MockSkyfiServer::feasibility_task_response())
            .expect("feasibility response should deserialize");
    assert_eq!(feasibility.id, "550e8400-e29b-41d4-a716-446655440030");
    assert!(feasibility.overall_score.is_some());

    let pass_prediction: PassPredictionResponse =
        serde_json::from_value(MockSkyfiServer::pass_prediction_response())
            .expect("pass prediction response should deserialize");
    assert_eq!(pass_prediction.passes.len(), 1);
}

#[test]
fn official_archive_schema_fields_are_present_on_archive_payloads() {
    let contract = OpenApiContract::load();
    let payload = MockSkyfiServer::archives_response();
    let archive = payload["archives"][0]
        .as_object()
        .expect("mock archive payload should be an object");
    let property_names = contract.property_names("ArchiveResponse");
    let required_names = contract.required_names("ArchiveResponse");

    for required in &required_names {
        assert!(
            archive.contains_key(required),
            "archive payload is missing required field {required}"
        );
    }

    for key in archive.keys() {
        assert!(
            property_names.contains(key),
            "archive payload contains unknown field {key}"
        );
    }
}
