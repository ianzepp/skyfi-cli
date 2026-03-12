use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- Enums ---

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApiProvider {
    Siwei,
    Satellogic,
    Umbra,
    Geosat,
    #[serde(rename = "SENTINEL1_CREODIAS")]
    Sentinel1Creodias,
    #[serde(rename = "SENTINEL2")]
    Sentinel2,
    #[serde(rename = "SENTINEL2_CREODIAS")]
    Sentinel2Creodias,
    Planet,
    Impro,
    #[serde(rename = "URBAN_SKY")]
    UrbanSky,
    Nsl,
    Vexcel,
    #[serde(rename = "ICEYE_US")]
    IceyeUs,
}

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProductType {
    Day,
    Night,
    Video,
    Sar,
    Hyperspectral,
    Multispectral,
    Stereo,
    Basemap,
}

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderType {
    Archive,
    Tasking,
}

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeliveryStatus {
    Created,
    Started,
    PaymentFailed,
    PlatformFailed,
    ProviderPending,
    ProviderComplete,
    ProviderFailed,
    ProcessingPending,
    ProcessingComplete,
    ProcessingFailed,
    DeliveryPending,
    DeliveryCompleted,
    DeliveryFailed,
    InternalImageProcessingPending,
}

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeliveryDriver {
    Gs,
    S3,
    Azure,
    DeliveryConfig,
    S3ServiceAccount,
    GsServiceAccount,
    AzureServiceAccount,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum)]
pub enum DeliverableType {
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "payload")]
    Payload,
    #[serde(rename = "cog")]
    Cog,
    #[serde(rename = "baba")]
    Baba,
}

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum)]
pub enum SarPolarisation {
    #[serde(rename = "HH")]
    Hh,
    #[serde(rename = "VV")]
    Vv,
}

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum)]
pub enum SarProductType {
    #[serde(rename = "GEC")]
    Gec,
    #[serde(rename = "SICD")]
    Sicd,
    #[serde(rename = "SIDD")]
    Sidd,
    #[serde(rename = "CPHD")]
    Cphd,
}

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum)]
pub enum SortColumn {
    #[serde(rename = "created_at")]
    CreatedAt,
    #[serde(rename = "last_modified")]
    LastModified,
    #[serde(rename = "customer_item_cost")]
    CustomerItemCost,
    #[serde(rename = "status")]
    Status,
}

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum)]
pub enum SortDirection {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum)]
pub enum FeasibilityCheckStatus {
    #[serde(rename = "PENDING")]
    Pending,
    #[serde(rename = "STARTED")]
    Started,
    #[serde(rename = "COMPLETE")]
    Complete,
    #[serde(rename = "ERROR")]
    Error,
}

// --- Request types ---

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetArchivesRequest {
    pub aoi: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_cloud_coverage_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_off_nadir_angle: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolutions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_types: Option<Vec<ProductType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<Vec<ApiProvider>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_data: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_overlap_ratio: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<i64>,
    pub page_size: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveOrderRequest {
    pub aoi: String,
    pub archive_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_driver: Option<DeliveryDriver>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_params: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskingOrderRequest {
    pub aoi: String,
    pub window_start: String,
    pub window_end: String,
    pub product_type: ProductType,
    pub resolution: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_item: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_cloud_coverage_percent: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_off_nadir_angle: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_provider: Option<ApiProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_driver: Option<DeliveryDriver>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_params: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sar_product_types: Option<Vec<SarProductType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sar_polarisation: Option<SarPolarisation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_window_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNotificationRequest {
    pub aoi: String,
    pub webhook_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gsd_min: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gsd_max: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_type: Option<ProductType>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aoi: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PassPredictionRequest {
    pub aoi: String,
    pub from_date: String,
    pub to_date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_types: Option<Vec<ProductType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolutions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_off_nadir_angle: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeasibilityRequest {
    pub aoi: String,
    pub product_type: ProductType,
    pub resolution: String,
    pub start_date: String,
    pub end_date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_cloud_coverage_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_item: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_provider: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderRedeliveryRequest {
    pub delivery_driver: DeliveryDriver,
    pub delivery_params: HashMap<String, serde_json::Value>,
}

// --- Response types ---

#[derive(Debug, Serialize, Deserialize)]
pub struct PongResponse {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhoamiUser {
    pub id: String,
    pub organization_id: Option<String>,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub is_demo_account: Option<bool>,
    pub current_budget_usage: i64,
    pub budget_amount: i64,
    pub has_valid_shared_card: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Archive {
    pub archive_id: String,
    pub provider: ApiProvider,
    pub constellation: String,
    pub product_type: ProductType,
    pub platform_resolution: f64,
    pub resolution: String,
    pub capture_timestamp: String,
    pub cloud_coverage_percent: Option<f64>,
    pub off_nadir_angle: Option<f64>,
    pub footprint: String,
    pub min_sq_km: f64,
    pub max_sq_km: f64,
    pub price_for_one_square_km: f64,
    pub price_for_one_square_km_cents: i64,
    pub price_full_scene: f64,
    pub total_area_square_km: f64,
    pub gsd: f64,
    pub open_data: Option<bool>,
    pub tiles_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveResponse {
    #[serde(flatten)]
    pub archive: Archive,
    pub overlap_ratio: f64,
    pub overlap_sqkm: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetArchivesResponse {
    pub request: serde_json::Value,
    pub archives: Vec<ArchiveResponse>,
    pub next_page: Option<String>,
    pub total: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderSummary {
    pub id: String,
    pub order_type: OrderType,
    pub order_cost: i64,
    pub owner_id: String,
    pub status: DeliveryStatus,
    pub aoi_sqkm: f64,
    pub order_code: String,
    pub created_at: String,
    pub order_id: String,
    pub item_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOrdersResponse {
    pub request: serde_json::Value,
    pub total: i64,
    pub orders: Vec<OrderSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationResponse {
    pub id: String,
    pub owner_id: String,
    pub aoi: String,
    pub gsd_min: Option<i64>,
    pub gsd_max: Option<i64>,
    pub product_type: Option<ProductType>,
    pub webhook_url: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationWithHistoryResponse {
    #[serde(flatten)]
    pub notification: NotificationResponse,
    pub history: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListNotificationsResponse {
    pub total: i64,
    pub notifications: Vec<NotificationResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeasibilityTaskResponse {
    pub id: String,
    pub valid_until: String,
    pub overall_score: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassPredictionResponse {
    pub passes: Vec<serde_json::Value>,
}
