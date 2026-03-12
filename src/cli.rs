use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::types::*;

#[derive(Debug, Parser)]
#[command(
    name = "skyfi",
    about = "CLI for the SkyFi Platform API (v2.0). SkyFi is a satellite imagery marketplace \
that provides on-demand access to 13+ satellite providers (Planet, Umbra, Satellogic, etc.) \
with pay-as-you-go pricing — search archives, task new captures, and download imagery \
through a single unified API.",
    after_long_help = "\
OVERVIEW:
  SkyFi provides access to satellite imagery from 13+ providers (Planet, Umbra,
  Satellogic, etc.) through a single API. This CLI exposes every Platform API
  endpoint as a subcommand.

AUTHENTICATION:
  Set your API key once:  skyfi config set-key <YOUR_KEY>
  Or export:              SKYFI_API_KEY=<YOUR_KEY>
  Verify it works:        skyfi whoami

CORE WORKFLOW:
  1. Search archive imagery for a location:
       skyfi archives search --aoi 'POLYGON ((lon1 lat1, lon2 lat2, lon3 lat3, lon1 lat1))'
  2. Inspect a result:
       skyfi archives get <ARCHIVE_ID>
  3. Order it:
       skyfi orders order-archive --aoi '<WKT_POLYGON>' --archive-id <ARCHIVE_ID>
  4. Track the order:
       skyfi orders get <ORDER_ID>
  5. Download when delivered:
       skyfi orders download <ORDER_ID>

TASKING WORKFLOW (request a new satellite capture):
  1. Check if a capture is feasible:
       skyfi feasibility check --aoi '<WKT_POLYGON>' --product-type day --resolution HIGH \\
         --start-date 2025-04-01 --end-date 2025-04-15
  2. Or find specific satellite passes:
       skyfi feasibility pass-prediction --aoi '<WKT_GEOMETRY>' \\
         --from-date 2025-04-01 --to-date 2025-04-07
  3. Place the tasking order:
       skyfi orders order-tasking --aoi '<WKT_POLYGON>' --product-type day --resolution HIGH \\
         --window-start 2025-04-01T00:00:00Z --window-end 2025-04-15T00:00:00Z

MONITORING WORKFLOW (get notified when new imagery appears):
  skyfi notifications create --aoi '<WKT_POLYGON>' --webhook-url https://example.com/hook

AOI FORMAT:
  All --aoi flags accept Well-Known Text (WKT). Typical examples:
    POLYGON ((lon1 lat1, lon2 lat2, lon3 lat3, lon1 lat1))
    POINT (lon lat)

OUTPUT:
  Human-readable by default. Use --json for structured JSON output suitable
  for parsing with jq or programmatic consumption.

TIPS:
  - Use --json on any command to get machine-parseable output
  - Pipe to jq for field extraction: skyfi orders list --json | jq '.orders[].orderId'
  - All dates are ISO 8601: 2025-04-01 or 2025-04-01T00:00:00Z
  - Comma-separated lists: --product-types day,sar --providers planet,umbra
  - Budget is in cents: divide orderCost by 100 for USD"
)]
pub struct Cli {
    /// Path to config file [default: ~/.config/skyfi/config.toml]
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// HTTP request timeout in seconds
    #[arg(long, global = true, default_value = "30")]
    pub timeout: u64,

    /// Emit JSON output instead of human-readable text. Use on any command
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Store and display API key and base URL settings
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Send a ping to the API and print the response. Use to verify connectivity and auth
    Ping,

    /// Show the authenticated user's email, org, and remaining budget
    Whoami,

    /// Search the satellite image archive catalog and retrieve image metadata
    #[command(after_long_help = "\
USAGE PATTERN:
  Search returns a paginated list of archive images matching your AOI and filters.
  Each result includes an archiveId, provider, resolution, capture date, area, and price.

  Typical flow:
    skyfi archives search --aoi '<WKT_POLYGON>' --from 2024-01-01 --max-cloud 20
    skyfi archives get <ARCHIVE_ID>

  Then order via: skyfi orders order-archive --aoi '<WKT_POLYGON>' --archive-id <ARCHIVE_ID>

FILTERING TIPS:
  - Combine --max-cloud and --max-nadir to get clearer, less distorted images
  - Use --product-types sar for all-weather radar imagery (ignores cloud cover)
  - Use --min-overlap 0.8 to ensure the image covers at least 80% of your AOI
  - Use --open-data true to find free Sentinel-2 imagery")]
    Archives {
        #[command(subcommand)]
        action: ArchivesAction,
    },

    /// Create, list, inspect, and download satellite imagery orders
    #[command(after_long_help = "\
ORDER TYPES:
  archive  - Purchase an existing image from the catalog (already captured)
  tasking  - Request a new satellite capture over your AOI in a future time window

LIFECYCLE:
  CREATED -> STARTED -> PROVIDER_PENDING -> PROVIDER_COMPLETE ->
  PROCESSING_PENDING -> PROCESSING_COMPLETE -> DELIVERY_PENDING -> DELIVERY_COMPLETED

  Failed states: PAYMENT_FAILED, PROVIDER_FAILED, PROCESSING_FAILED, DELIVERY_FAILED

TYPICAL FLOW:
  skyfi orders order-archive --aoi '<WKT_POLYGON>' --archive-id <ID>
  skyfi orders list --order-type archive
  skyfi orders get <ORDER_ID>
  skyfi orders download <ORDER_ID>
  skyfi orders download <ORDER_ID> --deliverable-type cog

DELIVERY DRIVERS:
  gs, s3, azure - deliver to your cloud storage bucket
  gs-service-account, s3-service-account, azure-service-account - use service account credentials
  none - download via the API (default)

COST:
  orderCost is in cents (USD). Divide by 100 for dollars.")]
    Orders {
        #[command(subcommand)]
        action: OrdersAction,
    },

    /// Set up webhook notifications when new imagery appears over an AOI
    #[command(after_long_help = "\
USAGE PATTERN:
  Create a notification to receive a POST to your webhook URL whenever new archive
  imagery matching your filters becomes available over your AOI.

  skyfi notifications create --aoi '<WKT_POLYGON>' --webhook-url https://example.com/hook
  skyfi notifications list
  skyfi notifications get <ID>
  skyfi notifications delete <ID>

  Optionally filter by GSD range (ground sample distance in meters) or product type.")]
    Notifications {
        #[command(subcommand)]
        action: NotificationsAction,
    },

    /// Check capture feasibility and predict satellite passes over a location
    #[command(after_long_help = "\
FEASIBILITY vs PASS PREDICTION:
  'check' asks: can satellites capture this AOI at this resolution in this date range?
    Returns a feasibility score (0-1) combining weather forecast and provider availability.
    The response includes a task ID; poll with 'status' until COMPLETE.

  'pass-prediction' asks: which specific satellites will pass over this point and when?
    Returns a list of passes with provider, satellite, date, off-nadir angle, and pricing.
    Use a pass's providerWindowId to lock a tasking order to that exact pass.

TYPICAL FLOW:
  skyfi feasibility check --aoi '<WKT_POLYGON>' --product-type day --resolution HIGH \\
    --start-date 2025-04-01 --end-date 2025-04-15
  skyfi feasibility status <FEASIBILITY_ID>

  skyfi feasibility pass-prediction --aoi '<WKT_GEOMETRY>' \\
    --from-date 2025-04-01 --to-date 2025-04-07
  skyfi orders order-tasking ... --provider-window-id <UUID_FROM_PASS>")]
    Feasibility {
        #[command(subcommand)]
        action: FeasibilityAction,
    },

    /// Get pricing tiers for all providers. Optionally scope to an AOI for area-based pricing
    #[command(after_long_help = "\
USAGE:
  skyfi pricing                         # all provider pricing tiers
  skyfi pricing --aoi '<WKT_POLYGON>'   # pricing calculated for your AOI's area")]
    Pricing {
        /// WKT geometry to calculate area-specific pricing. Omit for general pricing tiers
        #[arg(long)]
        aoi: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Print the current config file contents (base URL, API key presence)
    Show,

    /// Save an API key to the config file. Get your key from https://app.skyfi.com
    SetKey {
        /// Your SkyFi Platform API key
        key: String,
    },

    /// Override the API base URL (default: https://app.skyfi.com/platform-api)
    SetUrl {
        /// Full base URL including /platform-api path
        url: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum ArchivesAction {
    /// Search the catalog for existing satellite images over an area of interest
    #[command(after_long_help = "\
EXAMPLE:
  skyfi archives search \\
    --aoi 'POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))' \\
    --from 2024-06-01 --to 2024-12-31 \\
    --max-cloud 15 --product-types day --page-size 10")]
    Search {
        /// WKT geometry defining the search area. Usually a POLYGON.
        #[arg(long)]
        aoi: String,

        /// Only return images captured on or after this date (ISO 8601, e.g. 2024-01-01)
        #[arg(long)]
        from: Option<String>,

        /// Only return images captured on or before this date (ISO 8601)
        #[arg(long)]
        to: Option<String>,

        /// Exclude images with cloud cover above this percentage (0-100). Only affects optical imagery; SAR is unaffected by clouds
        #[arg(long)]
        max_cloud: Option<f64>,

        /// Exclude images with off-nadir angle above this value in degrees (0-50). Lower values mean the satellite was looking more straight down, producing less geometric distortion
        #[arg(long)]
        max_nadir: Option<f64>,

        /// Comma-separated imagery types to include: day, night, video, sar, hyperspectral, multispectral, stereo, basemap
        #[arg(long, value_delimiter = ',')]
        product_types: Option<Vec<ProductType>>,

        /// Comma-separated satellite providers to include: planet, umbra, satellogic, siwei, geosat, sentinel2, iceye-us, etc.
        #[arg(long, value_delimiter = ',')]
        providers: Option<Vec<ApiProvider>>,

        /// Comma-separated resolution tiers: LOW, MEDIUM, HIGH, VERY HIGH, SUPER HIGH, ULTRA HIGH, or SAR-specific: SPOT, STRIP, SCAN, DWELL, SLEA
        #[arg(long, value_delimiter = ',')]
        resolutions: Option<Vec<String>>,

        /// If true, only return freely available imagery (e.g. Sentinel-2)
        #[arg(long)]
        open_data: Option<bool>,

        /// Minimum fraction of your AOI that the image footprint must cover (0.0 to 1.0). Use 0.9+ to ensure near-complete coverage
        #[arg(long)]
        min_overlap: Option<f64>,

        /// Zero-based page number for pagination
        #[arg(long)]
        page: Option<i64>,

        /// Number of results per page
        #[arg(long, default_value = "25")]
        page_size: i64,
    },

    /// Retrieve full metadata for a single archive image by its ID
    #[command(after_long_help = "\
EXAMPLE:
  skyfi archives get abc123-def456

  Returns: provider, constellation, resolution, GSD, capture date, cloud cover,
  off-nadir angle, footprint geometry, area, and pricing.")]
    Get {
        /// The archiveId from a search result
        archive_id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum OrdersAction {
    /// List your orders with optional filtering and sorting
    #[command(after_long_help = "\
EXAMPLE:
  skyfi orders list --order-type archive --sort-by created-at --sort-dir desc --page-size 5")]
    List {
        /// Show only archive or tasking orders
        #[arg(long)]
        order_type: Option<OrderType>,

        /// Comma-separated sort columns: created-at, last-modified, customer-item-cost, status
        #[arg(long, value_delimiter = ',')]
        sort_by: Option<Vec<SortColumn>>,

        /// Comma-separated sort directions matching --sort-by columns: asc, desc
        #[arg(long, value_delimiter = ',')]
        sort_dir: Option<Vec<SortDirection>>,

        /// Zero-based page number
        #[arg(long)]
        page: Option<i64>,

        /// Results per page
        #[arg(long, default_value = "25")]
        page_size: i64,
    },

    /// Get full details and status history for a single order
    #[command(after_long_help = "\
EXAMPLE:
  skyfi orders get 550e8400-e29b-41d4-a716-446655440000")]
    Get {
        /// The orderId (UUID) returned when the order was created
        order_id: String,
    },

    /// Purchase an existing archive image. Requires an archiveId from 'archives search'
    #[command(after_long_help = "\
EXAMPLE:
  skyfi orders order-archive \\
    --aoi 'POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))' \\
    --archive-id abc123-def456 --label 'SF Bay Q4 2024'

  The AOI clips the archive image to your area of interest. You are charged
  based on the clipped area (sqkm * pricePerSqKm), not the full scene.")]
    OrderArchive {
        /// WKT polygon to clip the archive image. You pay for this area only
        #[arg(long)]
        aoi: String,

        /// The archiveId to order, from 'archives search' or 'archives get'
        #[arg(long)]
        archive_id: String,

        /// Human-readable label for this order
        #[arg(long)]
        label: Option<String>,

        /// Where to deliver the imagery: gs, s3, azure, or their service-account variants. Omit for API download
        #[arg(long)]
        delivery_driver: Option<DeliveryDriver>,

        /// URL to receive POST callbacks as order status changes
        #[arg(long)]
        webhook_url: Option<String>,
    },

    /// Request a new satellite capture over your AOI in a future time window
    #[command(after_long_help = "\
EXAMPLE:
  skyfi orders order-tasking \\
    --aoi 'POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))' \\
    --window-start 2025-04-01T00:00:00Z --window-end 2025-04-15T00:00:00Z \\
    --product-type day --resolution HIGH --max-cloud 20

  Use 'feasibility check' first to assess likelihood of successful capture.
  Use 'feasibility pass-prediction' to find specific satellite passes, then
  pass --provider-window-id to lock this order to an exact pass.")]
    OrderTasking {
        /// WKT polygon defining the area to capture
        #[arg(long)]
        aoi: String,

        /// Earliest acceptable capture time (ISO 8601 with timezone, e.g. 2025-04-01T00:00:00Z)
        #[arg(long)]
        window_start: String,

        /// Latest acceptable capture time (ISO 8601 with timezone)
        #[arg(long)]
        window_end: String,

        /// Imagery type to capture: day, night, video, sar, hyperspectral, multispectral, stereo
        #[arg(long)]
        product_type: ProductType,

        /// Resolution tier: LOW, MEDIUM, HIGH, VERY HIGH, SUPER HIGH, ULTRA HIGH, or SAR-specific tiers
        #[arg(long)]
        resolution: String,

        /// Human-readable label for this order
        #[arg(long)]
        label: Option<String>,

        /// If true, prioritize this capture (higher cost, higher likelihood of capture)
        #[arg(long)]
        priority: Option<bool>,

        /// Reject captures with cloud coverage above this percentage (0-100). Only meaningful for optical (day/night) imagery
        #[arg(long)]
        max_cloud: Option<i64>,

        /// Reject captures with off-nadir angle above this value in degrees (0-45). Lower = less distortion
        #[arg(long)]
        max_nadir: Option<i64>,

        /// Force a specific satellite provider for the capture
        #[arg(long)]
        required_provider: Option<ApiProvider>,

        /// Where to deliver the imagery. Omit for API download
        #[arg(long)]
        delivery_driver: Option<DeliveryDriver>,

        /// URL to receive POST callbacks as order status changes
        #[arg(long)]
        webhook_url: Option<String>,

        /// Lock this order to a specific satellite pass. UUID from 'feasibility pass-prediction' results
        #[arg(long)]
        provider_window_id: Option<String>,
    },

    /// Predict passes, select a pass, and create a tasking order pinned to that pass
    #[command(after_long_help = "\
EXAMPLE:
  skyfi orders pass-targeted \\
    --aoi 'POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))' \\
    --window-start 2025-04-01T00:00:00Z --window-end 2025-04-15T00:00:00Z \\
    --product-type day --resolution HIGH

This command:
  1. Calls 'feasibility pass-prediction' using your AOI, product type, resolution, and window
  2. Selects the earliest matching pass unless you provide --provider-window-id
  3. Creates 'orders order-tasking' pinned to that pass")]
    PassTargeted {
        /// WKT polygon defining the area to capture
        #[arg(long)]
        aoi: String,

        /// Earliest acceptable capture time (ISO 8601 with timezone, e.g. 2025-04-01T00:00:00Z)
        #[arg(long)]
        window_start: String,

        /// Latest acceptable capture time (ISO 8601 with timezone)
        #[arg(long)]
        window_end: String,

        /// Imagery type to capture: day, night, video, sar, hyperspectral, multispectral, stereo
        #[arg(long)]
        product_type: ProductType,

        /// Resolution tier: LOW, MEDIUM, HIGH, VERY HIGH, SUPER HIGH, ULTRA HIGH, or SAR-specific tiers
        #[arg(long)]
        resolution: String,

        /// Human-readable label for this order
        #[arg(long)]
        label: Option<String>,

        /// If true, prioritize this capture (higher cost, higher likelihood of capture)
        #[arg(long)]
        priority: Option<bool>,

        /// Reject captures with cloud coverage above this percentage (0-100). Only meaningful for optical (day/night) imagery
        #[arg(long)]
        max_cloud: Option<i64>,

        /// Exclude predicted or requested passes with off-nadir angle above this value in degrees
        #[arg(long)]
        max_nadir: Option<i64>,

        /// Force a specific satellite provider for the capture
        #[arg(long)]
        required_provider: Option<ApiProvider>,

        /// Where to deliver the imagery. Omit for API download
        #[arg(long)]
        delivery_driver: Option<DeliveryDriver>,

        /// URL to receive POST callbacks as order status changes
        #[arg(long)]
        webhook_url: Option<String>,

        /// Use this exact providerWindowId instead of auto-selecting the earliest predicted pass
        #[arg(long)]
        provider_window_id: Option<String>,
    },

    /// Get a download URL for an order's deliverable. Prints the redirect URL
    #[command(after_long_help = "\
DELIVERABLE TYPES:
  image   - Processed image file (default)
  payload - Raw sensor data package
  cog     - Cloud-Optimized GeoTIFF (efficient for streaming/tiling)
  baba    - Provider-specific format

EXAMPLE:
  skyfi orders download <ORDER_ID>
  skyfi orders download <ORDER_ID> --deliverable-type cog")]
    Download {
        /// The orderId of a completed order
        order_id: String,

        /// Which deliverable to download: image (default), payload, cog, or baba
        #[arg(long, default_value = "image")]
        deliverable_type: DeliverableType,
    },

    /// Re-deliver an order's imagery to a different destination (e.g. new S3 bucket)
    #[command(after_long_help = "\
EXAMPLE:
  skyfi orders redeliver <ORDER_ID> \\
    --delivery-driver s3 \\
    --delivery-params '{\"bucket\":\"my-bucket\",\"prefix\":\"imagery/\"}'")]
    Redeliver {
        /// The orderId to redeliver
        order_id: String,

        /// Delivery destination: gs, s3, azure, or their service-account variants
        #[arg(long)]
        delivery_driver: DeliveryDriver,

        /// JSON object with driver-specific params (e.g. bucket, prefix, container name)
        #[arg(long)]
        delivery_params: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum NotificationsAction {
    /// List all active notifications with their AOIs and webhook URLs
    #[command(after_long_help = "\
EXAMPLE:
  skyfi notifications list --page-size 10")]
    List {
        /// Zero-based page number
        #[arg(long)]
        page: Option<i64>,

        /// Results per page
        #[arg(long, default_value = "25")]
        page_size: Option<i64>,
    },

    /// Get a notification's config and its event history (past webhook deliveries)
    Get {
        /// The notification ID from 'notifications list' or 'notifications create'
        notification_id: String,
    },

    /// Create a webhook notification that fires when new imagery appears over your AOI
    #[command(after_long_help = "\
EXAMPLE:
  skyfi notifications create \\
    --aoi 'POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))' \\
    --webhook-url https://example.com/new-imagery \\
    --product-type day --gsd-max 5

GSD (Ground Sample Distance):
  The size in meters that one pixel represents on the ground.
  Lower GSD = higher resolution. --gsd-max 5 means only notify for images
  where each pixel covers 5m or less.")]
    Create {
        /// WKT polygon defining the area to monitor
        #[arg(long)]
        aoi: String,

        /// URL that will receive a POST request when matching imagery appears
        #[arg(long)]
        webhook_url: String,

        /// Minimum ground sample distance in meters. Only notify for imagery coarser than this
        #[arg(long)]
        gsd_min: Option<i64>,

        /// Maximum ground sample distance in meters. Only notify for imagery finer (higher-res) than this
        #[arg(long)]
        gsd_max: Option<i64>,

        /// Only notify for this imagery type: day, night, video, sar, hyperspectral, multispectral, stereo
        #[arg(long)]
        product_type: Option<ProductType>,
    },

    /// Delete an active notification. Stops all future webhook deliveries for it
    Delete {
        /// The notification ID to delete
        notification_id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum FeasibilityAction {
    /// Submit an asynchronous feasibility check. Returns a task ID to poll with 'status'
    #[command(after_long_help = "\
The feasibility score (0-1) combines weather forecast and satellite provider availability.
The task starts in PENDING state; poll with 'feasibility status <ID>' until COMPLETE.

EXAMPLE:
  skyfi feasibility check \\
    --aoi 'POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))' \\
    --product-type day --resolution HIGH \\
    --start-date 2025-04-01 --end-date 2025-04-15 --max-cloud 20")]
    Check {
        /// WKT geometry for the area to evaluate
        #[arg(long)]
        aoi: String,

        /// Imagery type: day, night, video, sar, hyperspectral, multispectral, stereo
        #[arg(long)]
        product_type: ProductType,

        /// Resolution tier: LOW, MEDIUM, HIGH, VERY HIGH, SUPER HIGH, ULTRA HIGH
        #[arg(long)]
        resolution: String,

        /// Beginning of the capture window (ISO 8601 date, e.g. 2025-04-01)
        #[arg(long)]
        start_date: String,

        /// End of the capture window (ISO 8601 date)
        #[arg(long)]
        end_date: String,

        /// Max acceptable cloud coverage percentage (0-100). Affects the weather component of the feasibility score
        #[arg(long)]
        max_cloud: Option<f64>,

        /// If true, evaluate as a priority tasking (faster but more expensive)
        #[arg(long)]
        priority: Option<bool>,

        /// Only evaluate this provider's satellites. Currently only PLANET or UMBRA supported
        #[arg(long)]
        required_provider: Option<String>,
    },

    /// Poll the status of a feasibility check by its task ID. States: PENDING, STARTED, COMPLETE, ERROR
    Status {
        /// The feasibility task ID returned by 'feasibility check'
        feasibility_id: String,
    },

    /// Find specific satellite passes over a location in a date range. Returns pass times, angles, and pricing
    #[command(after_long_help = "\
Each pass in the response includes: provider, satellite name/ID, pass date, off-nadir angle,
solar elevation angle, resolution, GSD range, min/max order area, and price per sqkm.

Use a pass's providerWindowId with 'orders order-tasking --provider-window-id' to lock
your tasking order to that exact satellite pass.

EXAMPLE:
  skyfi feasibility pass-prediction \\
    --aoi 'POINT (-122.4 37.7)' \\
    --from-date 2025-04-01 --to-date 2025-04-07 \\
    --product-types day,sar --max-nadir 30")]
    PassPrediction {
        /// WKT geometry (usually POINT or POLYGON) for the location to predict passes over
        #[arg(long)]
        aoi: String,

        /// Start of the prediction window (ISO 8601 date)
        #[arg(long)]
        from_date: String,

        /// End of the prediction window (ISO 8601 date)
        #[arg(long)]
        to_date: String,

        /// Comma-separated imagery types: day, night, video, sar, hyperspectral, multispectral, stereo
        #[arg(long, value_delimiter = ',')]
        product_types: Option<Vec<ProductType>>,

        /// Comma-separated resolution tiers to filter passes
        #[arg(long, value_delimiter = ',')]
        resolutions: Option<Vec<String>>,

        /// Exclude passes with off-nadir angle above this value in degrees
        #[arg(long)]
        max_nadir: Option<f64>,
    },
}
