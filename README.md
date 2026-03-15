# skyfi-cli

A command-line interface for the [SkyFi Platform API](https://app.skyfi.com), written in Rust.

SkyFi is a satellite imagery marketplace that aggregates 13+ satellite providers — including Planet,
Umbra, Satellogic, Siwei, Geosat, ICEYE, Vexcel, and Sentinel — behind a single unified API with
pay-as-you-go pricing. This CLI exposes the full Platform API v2 command surface, covering archive
search, archive and tasking orders, feasibility checks, pass prediction, notifications, pricing,
and a prompt-driven research-agent mode that writes markdown briefs.

The project and package are named `skyfi-cli`, but the installed executable is `skyfi`.

---

## Table of Contents

- [Installation](#installation)
- [Configuration](#configuration)
- [Concepts](#concepts)
- [Workflows](#workflows)
  - [Archive Workflow](#archive-workflow)
  - [Tasking Workflow](#tasking-workflow)
  - [Pass-Targeted Tasking Workflow](#pass-targeted-tasking-workflow)
  - [Monitoring Workflow](#monitoring-workflow)
- [Command Reference](#command-reference)
  - [Global Flags](#global-flags)
  - [config](#config)
  - [ping](#ping)
  - [whoami](#whoami)
  - [archives](#archives)
  - [orders](#orders)
  - [feasibility](#feasibility)
  - [notifications](#notifications)
  - [alerts](#alerts)
  - [pricing](#pricing)
  - [research](#research)
- [Output and JSON Mode](#output-and-json-mode)
- [Error Handling](#error-handling)
- [Development](#development)

---

## Installation

### Homebrew (macOS and Linux x86_64)

```bash
brew install ianzepp/tap/skyfi
```

### Shell script

Downloads the latest release binary for your platform directly from GitHub Releases:

```bash
curl -fsSL https://raw.githubusercontent.com/ianzepp/skyfi-cli/master/install.sh | bash
```

Supported targets: macOS (`x86_64`, `arm64`) and Linux (`x86_64`). Linux arm64 is not currently
supported. The install script puts the binary in `/usr/local/bin` by default; override this with
the `INSTALL_DIR` environment variable:

```bash
INSTALL_DIR=~/.local/bin curl -fsSL .../install.sh | bash
```

### Build from source

Requires a [Rust toolchain](https://rustup.rs) (stable, edition 2021):

```bash
cargo install --path .
```

This installs the `skyfi` executable.

Or run without installing (useful during development):

```bash
cargo run -- --help
```

---

## Configuration

skyfi uses a TOML config file at `~/.config/skyfi/config.toml` by default. The file and its
parent directory are created automatically on first write.

### Set an API key

Get your API key from <https://app.skyfi.com>. Store it once:

```bash
skyfi config set-key <YOUR_KEY>
```

The key is stored in the config file. To avoid writing secrets to disk, use the environment
variable instead (takes precedence over the config file):

```bash
export SKYFI_API_KEY=<YOUR_KEY>
```

### Inspect current config

```bash
skyfi config show
```

The stored API key is always redacted in this output so it is safe to share or log.

### Override the base URL

Useful for pointing at a staging or self-hosted instance:

```bash
skyfi config set-url https://app.skyfi.com/platform-api
```

The URL is validated before it is saved.

### Use a custom config file

Pass `--config` on any command to use a different config file path:

```bash
skyfi --config ./my-config.toml whoami
```

---

## Concepts

### Area of Interest (AOI)

All `--aoi` flags accept [Well-Known Text (WKT)](https://en.wikipedia.org/wiki/Well-known_text_representation_of_geometry)
geometry strings. Coordinates are `longitude latitude` (X Y, not lat/lon):

```
POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))
POINT (-122.4 37.7)
```

For archive orders and tasking orders, the AOI clips the imagery to your area. You are charged
based on the clipped area in square kilometers, not the full scene footprint.

### Pricing and budget

All cost values in the API (and in `--json` output) are denominated in **cents (USD)**. Divide by
100 to get dollars. For example, `orderCost: 1250` means $12.50.

Your account budget and current usage are visible via `skyfi whoami`.

### Resolution tiers

Resolution tiers are named strings, not numeric GSD values. Common optical tiers:

| Tier | Approximate GSD |
|---|---|
| `LOW` | > 10 m/px |
| `MEDIUM` | 5–10 m/px |
| `HIGH` | 1–5 m/px |
| `VERY HIGH` | 0.5–1 m/px |
| `SUPER HIGH` | 30–50 cm/px |
| `ULTRA HIGH` | < 30 cm/px |

SAR-specific tiers: `SPOT`, `STRIP`, `SCAN`, `DWELL`, `SLEA`.

**GSD (Ground Sample Distance)** is the metric equivalent: the size in meters that one pixel
represents on the ground. Lower GSD = higher resolution.

### Product types

| Type | Description |
|---|---|
| `day` | Daytime optical imagery |
| `night` | Nighttime optical imagery |
| `sar` | Synthetic Aperture Radar (works through clouds and at night) |
| `multispectral` | Multiple spectral bands (RGB + NIR, etc.) |
| `hyperspectral` | Many narrow spectral bands for material identification |
| `stereo` | Stereo pair for 3D elevation extraction |
| `video` | Video capture |
| `basemap` | Mosaic basemap product |

### Providers

Available satellite providers: `planet`, `umbra`, `satellogic`, `siwei`, `geosat`, `sentinel2`,
`sentinel2-creodias`, `sentinel1-creodias`, `impro`, `urban-sky`, `nsl`, `vexcel`, `iceye-us`.

### Order lifecycle

Archive and tasking orders move through a defined set of states:

```
CREATED
  -> STARTED
  -> PROVIDER_PENDING
  -> PROVIDER_COMPLETE
  -> PROCESSING_PENDING
  -> PROCESSING_COMPLETE
  -> DELIVERY_PENDING
  -> DELIVERY_COMPLETED
```

Failure states at any stage: `PAYMENT_FAILED`, `PROVIDER_FAILED`, `PROCESSING_FAILED`,
`DELIVERY_FAILED`, `PLATFORM_FAILED`.

### Delivery drivers

By default, completed imagery can be downloaded via the API (`orders download`). You can also
have the platform push imagery directly to your cloud storage bucket:

| Driver | Description |
|---|---|
| `gs` | Google Cloud Storage |
| `s3` | Amazon S3 |
| `azure` | Azure Blob Storage |
| `gs-service-account` | GCS using a service account key |
| `s3-service-account` | S3 using IAM service account credentials |
| `azure-service-account` | Azure using a service principal |
| `delivery-config` | Use a pre-configured delivery profile on your account |
| `none` | No automatic delivery (API download only) |

---

## Workflows

### Archive Workflow

Search and purchase existing satellite imagery that has already been captured.

**Step 1 — Search for imagery over your area:**

```bash
skyfi archives search \
  --aoi 'POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))' \
  --from 2024-06-01 \
  --to 2024-12-31 \
  --max-cloud 20 \
  --product-types day \
  --page-size 10
```

Output columns: archive ID, provider, area (km²), GSD (m), price per km², capture date.

**Step 2 — Inspect a specific result:**

```bash
skyfi archives get <ARCHIVE_ID>
```

**Step 3 — Purchase the archive image:**

```bash
skyfi orders order-archive \
  --aoi 'POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))' \
  --archive-id <ARCHIVE_ID> \
  --label "SF Bay Area survey"
```

**Step 4 — Track and download:**

```bash
skyfi orders get <ORDER_ID>
skyfi orders download <ORDER_ID>
```

The `download` subcommand prints a redirect URL. Pipe it to curl to save the file:

```bash
curl -L "$(skyfi orders download <ORDER_ID>)" -o imagery.tif
```

To download a Cloud-Optimized GeoTIFF instead of the default processed image:

```bash
curl -L "$(skyfi orders download <ORDER_ID> --deliverable-type cog)" -o imagery.cog.tif
```

---

### Tasking Workflow

Commission a new satellite capture over your AOI within a future time window.

**Step 1 — Check feasibility first (optional but recommended):**

Feasibility analysis combines weather forecast probability and satellite provider availability to
produce a score between 0 and 1. The job runs asynchronously; use `--wait` to block until complete:

```bash
skyfi feasibility check \
  --aoi 'POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))' \
  --product-type day \
  --resolution HIGH \
  --start-date 2025-04-01 \
  --end-date 2025-04-15 \
  --max-cloud 20 \
  --wait
```

Without `--wait`, the command returns immediately with a task ID. Poll status yourself:

```bash
skyfi feasibility status <FEASIBILITY_ID>
```

**Step 2 — Place the tasking order:**

```bash
skyfi orders order-tasking \
  --aoi 'POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))' \
  --window-start 2025-04-01T00:00:00Z \
  --window-end 2025-04-15T00:00:00Z \
  --product-type day \
  --resolution HIGH \
  --max-cloud 20
```

---

### Pass-Targeted Tasking Workflow

For more control, inspect predicted satellite passes first and then lock your order to a specific
pass. This is useful when you need imagery from a particular satellite, angle, or exact time.

**Step 1 — List predicted passes:**

```bash
skyfi feasibility pass-prediction \
  --aoi 'POINT (-122.4 37.7)' \
  --from-date 2025-04-01 \
  --to-date 2025-04-07 \
  --product-types day,sar \
  --max-nadir 30
```

Output columns: provider, resolution tier, off-nadir angle, pass date, `providerWindowId`.

**Step 2 — Pin a tasking order to a specific pass:**

```bash
skyfi orders order-tasking \
  --aoi 'POLYGON ((...))' \
  --window-start 2025-04-01T00:00:00Z \
  --window-end 2025-04-07T00:00:00Z \
  --product-type day \
  --resolution HIGH \
  --provider-window-id <UUID_FROM_PASS>
```

**Shortcut — `orders pass-targeted`:**

The `pass-targeted` subcommand runs both steps atomically: it calls `pass-prediction`, selects the
earliest matching pass, and immediately places a tasking order pinned to that pass:

```bash
skyfi orders pass-targeted \
  --aoi 'POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))' \
  --window-start 2025-04-01T00:00:00Z \
  --window-end 2025-04-15T00:00:00Z \
  --product-type day \
  --resolution HIGH
```

To pick a specific pass rather than the earliest, supply `--provider-window-id`:

```bash
skyfi orders pass-targeted \
  --aoi 'POLYGON ((...))' \
  --window-start 2025-04-01T00:00:00Z \
  --window-end 2025-04-15T00:00:00Z \
  --product-type day \
  --resolution HIGH \
  --provider-window-id <UUID_FROM_PASS>
```

---

### Monitoring Workflow

Set up a webhook that fires whenever new imagery matching your filters becomes available over your
AOI. Useful for staying notified without repeated manual searches.

```bash
skyfi notifications create \
  --aoi 'POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))' \
  --webhook-url https://example.com/hooks/new-imagery \
  --product-type day \
  --gsd-max 5
```

`--gsd-max 5` means "only notify for imagery where each pixel covers 5 m or less" — i.e., high
resolution captures only.

Manage notifications:

```bash
skyfi notifications list
skyfi notifications get <NOTIFICATION_ID>
skyfi notifications delete <NOTIFICATION_ID>
```

Poll for unseen history events:

```bash
skyfi alerts poll
skyfi alerts watch --interval 300
skyfi alerts install --interval 300
```

---

## Command Reference

### Global Flags

These flags work on every command:

| Flag | Default | Description |
|---|---|---|
| `--config <PATH>` | `~/.config/skyfi/config.toml` | Path to a custom config file |
| `--timeout <SECONDS>` | `30` | HTTP request timeout in seconds |
| `--json` | off | Emit machine-readable JSON instead of human-readable text |

### config

Manage local CLI configuration. Does not require an API key.

#### `config show`

Print the current config file contents. The API key is always redacted.

```bash
skyfi config show
```

#### `config set-key <KEY>`

Save an API key to the config file. The key is written to `~/.config/skyfi/config.toml`.

```bash
skyfi config set-key sk_live_abc123...
```

#### `config set-url <URL>`

Override the API base URL. The URL is validated before saving.

```bash
skyfi config set-url https://app.skyfi.com/platform-api
```

---

### ping

Verify network connectivity and basic API availability. This command uses the normal authenticated
API client, so configure an API key first.

```bash
skyfi ping
```

---

### whoami

Show the authenticated user's name, email, user ID, organization, and budget status.

```bash
skyfi whoami
```

Example output:
```
Jane Smith <jane@example.com>
ID:       usr_abc123
Org:      org_xyz789
Budget:   $45.00 used of $500.00
```

---

### archives

Search and inspect the archive catalog of previously captured satellite imagery.

#### `archives search`

Search for existing imagery over an AOI. Returns a paginated list sorted by relevance.

```bash
skyfi archives search \
  --aoi 'POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))' \
  [--from <DATE>] \
  [--to <DATE>] \
  [--max-cloud <PERCENT>] \
  [--max-nadir <DEGREES>] \
  [--product-types <TYPE,...>] \
  [--providers <PROVIDER,...>] \
  [--resolutions <TIER,...>] \
  [--open-data <true|false>] \
  [--min-overlap <0.0-1.0>] \
  [--page <N>] \
  [--page-size <N>]
```

| Flag | Description |
|---|---|
| `--aoi` | WKT geometry for the search area (required) |
| `--from` | Only return images captured on or after this date (ISO 8601) |
| `--to` | Only return images captured on or before this date (ISO 8601) |
| `--max-cloud` | Exclude images with cloud cover above this percentage (0–100). Only affects optical imagery; SAR is unaffected by clouds |
| `--max-nadir` | Exclude images with off-nadir angle above this value in degrees (0–50). Lower values mean less geometric distortion |
| `--product-types` | Comma-separated product types to include: `day`, `night`, `sar`, `video`, `hyperspectral`, `multispectral`, `stereo`, `basemap` |
| `--providers` | Comma-separated providers to include |
| `--resolutions` | Comma-separated resolution tiers to include |
| `--open-data` | If `true`, return only freely available imagery (e.g. Sentinel-2) |
| `--min-overlap` | Minimum fraction of your AOI that the image footprint must cover (0.0–1.0) |
| `--page` | Zero-based page number for pagination |
| `--page-size` | Number of results per page (default: 25) |

The total result count is printed to stderr so it does not interfere with piping stdout.

#### `archives get <ARCHIVE_ID>`

Retrieve full metadata for a single archive image by its ID.

```bash
skyfi archives get abc123-def456
```

Returns: provider, constellation, resolution tier, GSD, capture timestamp, cloud cover,
off-nadir angle, area (km²), price per km², minimum and maximum orderable area, and footprint.

---

### orders

Create, list, inspect, and download satellite imagery orders.

#### `orders list`

List your orders with optional filtering and sorting.

```bash
skyfi orders list \
  [--order-type archive|tasking] \
  [--sort-by <COLUMN,...>] \
  [--sort-dir <asc|desc,...>] \
  [--page <N>] \
  [--page-size <N>]
```

Sort columns: `created-at`, `last-modified`, `customer-item-cost`, `status`. Multiple `--sort-by`
and `--sort-dir` values are paired positionally.

#### `orders get <ORDER_ID>`

Get full details and status history for a single order.

```bash
skyfi orders get 550e8400-e29b-41d4-a716-446655440000
```

#### `orders order-archive`

Purchase an existing archive image. The AOI clips the image to your area of interest; you pay
for the clipped area only.

```bash
skyfi orders order-archive \
  --aoi '<WKT_POLYGON>' \
  --archive-id <ARCHIVE_ID> \
  [--label <TEXT>] \
  [--delivery-driver <DRIVER>] \
  [--webhook-url <URL>]
```

| Flag | Description |
|---|---|
| `--aoi` | WKT polygon to clip the image (required) |
| `--archive-id` | The archive ID from `archives search` (required) |
| `--label` | Human-readable label for this order |
| `--delivery-driver` | Cloud storage driver for automatic delivery |
| `--webhook-url` | URL to receive POST callbacks on status changes |

#### `orders order-tasking`

Commission a new satellite capture over your AOI within a future time window.

```bash
skyfi orders order-tasking \
  --aoi '<WKT_POLYGON>' \
  --window-start <ISO8601_DATETIME> \
  --window-end <ISO8601_DATETIME> \
  --product-type <TYPE> \
  --resolution <TIER> \
  [--label <TEXT>] \
  [--priority] \
  [--max-cloud <PERCENT>] \
  [--max-nadir <DEGREES>] \
  [--required-provider <PROVIDER>] \
  [--delivery-driver <DRIVER>] \
  [--webhook-url <URL>] \
  [--provider-window-id <UUID>]
```

| Flag | Description |
|---|---|
| `--aoi` | WKT polygon defining the area to capture (required) |
| `--window-start` | Earliest acceptable capture time, ISO 8601 with timezone (required) |
| `--window-end` | Latest acceptable capture time, ISO 8601 with timezone (required) |
| `--product-type` | Imagery type (required) |
| `--resolution` | Resolution tier (required) |
| `--label` | Human-readable label |
| `--priority` | If set, prioritize this capture (higher cost, higher likelihood of success) |
| `--max-cloud` | Reject captures with cloud cover above this percentage |
| `--max-nadir` | Reject captures with off-nadir angle above this value in degrees |
| `--required-provider` | Force a specific satellite provider |
| `--delivery-driver` | Cloud storage driver for automatic delivery |
| `--webhook-url` | URL to receive POST callbacks on status changes |
| `--provider-window-id` | Lock this order to a specific predicted satellite pass (UUID from `feasibility pass-prediction`) |

#### `orders pass-targeted`

Run pass prediction, select a pass, and create a tasking order pinned to that pass in a single
command. Equivalent to running `feasibility pass-prediction` and `orders order-tasking
--provider-window-id` separately.

```bash
skyfi orders pass-targeted \
  --aoi '<WKT_POLYGON>' \
  --window-start <ISO8601_DATETIME> \
  --window-end <ISO8601_DATETIME> \
  --product-type <TYPE> \
  --resolution <TIER> \
  [--label <TEXT>] \
  [--priority] \
  [--max-cloud <PERCENT>] \
  [--max-nadir <DEGREES>] \
  [--required-provider <PROVIDER>] \
  [--delivery-driver <DRIVER>] \
  [--webhook-url <URL>] \
  [--provider-window-id <UUID>]
```

By default, selects the earliest predicted pass. Supply `--provider-window-id` to override.

#### `orders download <ORDER_ID>`

Get a download URL for a completed order's deliverable. Prints the URL to stdout.

```bash
skyfi orders download <ORDER_ID> [--deliverable-type image|payload|cog|baba]
```

| Deliverable | Description |
|---|---|
| `image` | Processed image file (default) |
| `payload` | Raw sensor data package |
| `cog` | Cloud-Optimized GeoTIFF (efficient for streaming and tiling) |
| `baba` | Provider-specific format |

Save to file:

```bash
curl -L "$(skyfi orders download <ORDER_ID>)" -o output.tif
```

#### `orders redeliver <ORDER_ID>`

Re-deliver an order's imagery to a different cloud storage destination.

```bash
skyfi orders redeliver <ORDER_ID> \
  --delivery-driver s3 \
  --delivery-params '{"bucket":"my-bucket","prefix":"imagery/"}'
```

---

### feasibility

Assess whether a new satellite capture is feasible and identify specific satellite passes.

#### `feasibility check`

Submit an asynchronous feasibility check. The score (0–1) combines weather forecast probability
and satellite provider availability for your parameters. Returns a task ID immediately; the job
runs in the background.

```bash
skyfi feasibility check \
  --aoi '<WKT_GEOMETRY>' \
  --product-type <TYPE> \
  --resolution <TIER> \
  --start-date <DATE> \
  --end-date <DATE> \
  [--max-cloud <PERCENT>] \
  [--priority] \
  [--required-provider planet|umbra] \
  [--wait]
```

| Flag | Description |
|---|---|
| `--aoi` | WKT geometry for the area to evaluate (required) |
| `--product-type` | Imagery type (required) |
| `--resolution` | Resolution tier (required) |
| `--start-date` | Beginning of the capture window, ISO 8601 date (required) |
| `--end-date` | End of the capture window, ISO 8601 date (required) |
| `--max-cloud` | Max acceptable cloud coverage percentage (affects the weather component of the score) |
| `--priority` | Evaluate as a priority tasking |
| `--required-provider` | Only evaluate this provider's satellites (currently `planet` or `umbra`) |
| `--wait` | Block until the task reaches `COMPLETE` or `ERROR`, polling every 2 seconds |

Task states: `PENDING` → `STARTED` → `COMPLETE` or `ERROR`.

#### `feasibility status <FEASIBILITY_ID>`

Poll the status of a feasibility check.

```bash
skyfi feasibility status <FEASIBILITY_ID>
```

#### `feasibility pass-prediction`

Find specific satellite passes over a location within a date range. Returns pass times, angles,
and pricing for each predicted overpass.

```bash
skyfi feasibility pass-prediction \
  --aoi '<WKT_GEOMETRY>' \
  --from-date <DATE> \
  --to-date <DATE> \
  [--product-types <TYPE,...>] \
  [--resolutions <TIER,...>] \
  [--max-nadir <DEGREES>]
```

Output columns: provider, resolution tier, off-nadir angle, pass date, `providerWindowId`.

The `providerWindowId` UUID identifies the specific satellite pass. Supply it to `orders
order-tasking --provider-window-id` or `orders pass-targeted --provider-window-id` to lock your
tasking order to that exact pass.

---

### notifications

Receive webhooks when new imagery appears over an AOI.

#### `notifications list`

List all active notifications.

```bash
skyfi notifications list [--page <N>] [--page-size <N>]
```

#### `notifications get <NOTIFICATION_ID>`

Get a notification's config and its event history (past webhook deliveries).

```bash
skyfi notifications get <NOTIFICATION_ID>
```

#### `notifications create`

Create a webhook notification that fires when new matching imagery is added to the archive.

```bash
skyfi notifications create \
  --aoi '<WKT_POLYGON>' \
  --webhook-url <URL> \
  [--product-type <TYPE>] \
  [--gsd-min <METERS>] \
  [--gsd-max <METERS>]
```

| Flag | Description |
|---|---|
| `--aoi` | WKT polygon for the area to monitor (required) |
| `--webhook-url` | URL to receive a POST request when matching imagery appears (required) |
| `--product-type` | Only notify for this imagery type |
| `--gsd-min` | Only notify for imagery with GSD above this value (coarser than this threshold) |
| `--gsd-max` | Only notify for imagery with GSD below this value (finer/higher-res than this threshold) |

#### `notifications delete <NOTIFICATION_ID>`

Delete a notification. Stops all future webhook deliveries.

```bash
skyfi notifications delete <NOTIFICATION_ID>
```

---

### alerts

Poll and track unseen notification history events directly from the SkyFi API.

These commands do not require the MCP server. They work by listing your notifications and then
reading each notification's `history` from `GET /notifications/{id}`. The CLI stores seen event
fingerprints in a local state file at `~/.config/skyfi/alerts-state.json` by default, or next to
the file passed with `--config`.

#### `alerts poll`

Fetch all notification history and print only unseen events.

```bash
skyfi alerts poll
skyfi alerts poll --json
skyfi alerts poll --no-save-state
```

| Flag | Description |
|---|---|
| `--no-save-state` | Show unseen events without recording them as seen |

#### `alerts watch`

Poll continuously on a fixed interval.

```bash
skyfi alerts watch --interval 300
```

| Flag | Description |
|---|---|
| `--interval` | Seconds between polls (default: 300) |
| `--no-save-state` | Show unseen events without recording them as seen |

#### `alerts install`

Install a local alert polling service that integrates with the host OS:

- macOS: `launchd` agent with native Notification Center banners via `osascript`
- Linux: `systemd --user` service + timer, with desktop notifications via `notify-send` when available

In both cases the service can optionally invoke a user hook for each new alert.

```bash
skyfi alerts install --interval 300
skyfi alerts install --interval 120 --on-alert ~/bin/skyfi-alert-hook.sh
skyfi alerts install --no-load
```

Installed artifacts:

- macOS: `~/Library/LaunchAgents/com.skyfi.alerts.plist`
- Linux: `~/.config/systemd/user/skyfi-alerts.service` and `~/.config/systemd/user/skyfi-alerts.timer`

The installed service:

- runs `skyfi alerts service-run` on the requested interval
- uses the normal `alerts-state.json` file so seen/unseen behavior stays consistent
- on macOS, writes stdout/stderr under `~/.config/skyfi/logs/`
- on Linux, can be managed with standard `systemctl --user` commands after install

When a new alert is found, the service:

- posts a local notification titled `SkyFi Alert`
- optionally spawns the executable passed to `--on-alert`

Hook behavior:

- the hook is executed once per new alert
- the full alert JSON is sent to the hook on stdin
- the following environment variables are set when available:
  - `SKYFI_ALERT_NOTIFICATION_ID`
  - `SKYFI_ALERT_WEBHOOK_URL`
  - `SKYFI_ALERT_EVENT_KEY`
  - `SKYFI_ALERT_PRODUCT_TYPE`
  - `SKYFI_ALERT_OBSERVED_AT`

| Flag | Description |
|---|---|
| `--interval` | Seconds between polling runs (default: 300) |
| `--on-alert` | Executable to run once per new alert; receives alert JSON on stdin |
| `--no-load` | Write the OS service files without loading/enabling them immediately |

#### `alerts state show`

Inspect the local alert polling state.

```bash
skyfi alerts state show
```

#### `alerts state reset`

Forget all previously seen events and start fresh on the next poll.

```bash
skyfi alerts state reset
```

---

### pricing

Get pricing tiers for all providers, optionally scoped to your AOI's area.

```bash
skyfi pricing
skyfi pricing --aoi '<WKT_POLYGON>'
```

Without `--aoi`, returns general pricing tiers for all providers. With `--aoi`, returns
area-specific pricing calculated against your polygon's area.

### research

Run a prompt-driven research loop that can resolve locations, inspect account readiness, search
archives, inspect archive detail, check pricing, run feasibility checks, and predict passes. The
command writes a markdown brief and can optionally emit a JSON trace.

```bash
skyfi research "<OBJECTIVE>" [--output <FILE.md>] [--trace-output <FILE.json>] [--model <MODEL>] [--max-steps <N>]
```

Requirements:

- `OPENAI_API_KEY` must be set
- `OPENAI_MODEL` can set the default model, or use `--model`
- `OPENAI_BASE_URL` can point to an OpenAI-compatible proxy endpoint

Behavior:

- human-readable mode streams model text and tool progress to the terminal
- `--json` suppresses live progress and prints the final artifact metadata as JSON

---

## Output and JSON Mode

By default, every command produces human-readable text output. Use `--json` on any command to
get structured JSON instead:

```bash
skyfi --json archives search --aoi 'POLYGON ((...))'
skyfi --json orders list
skyfi --json feasibility check ...
```

JSON output goes to stdout; informational messages (total counts, next-page hints) go to stderr.
This means you can safely pipe `--json` output to `jq` without mixing the two streams.

Examples with `jq`:

```bash
# Extract all archive IDs from a search
skyfi --json archives search --aoi 'POLYGON ((...))' | jq '.archives[].archiveId'

# List order IDs with their statuses
skyfi --json orders list | jq '.orders[] | {id: .orderId, status: .status}'

# Get just the download URL
skyfi --json orders download <ORDER_ID> | jq -r '.download_url'

# List pass providerWindowIds for tasking
skyfi --json feasibility pass-prediction --aoi 'POINT (...)' \
  --from-date 2025-04-01 --to-date 2025-04-07 | jq '.passes[].providerWindowId'
```

---

## Error Handling

Errors are printed to stderr with the prefix `error:` and the process exits with code 1.

Common error scenarios:

| Scenario | Message |
|---|---|
| No API key configured | `config: no API key configured. Set via 'skyfi config set-key <KEY>' or SKYFI_API_KEY env var` |
| Invalid API key | `api error (401): ...` |
| Network timeout | `http: error sending request for url ...` |
| Archive not found | `api error (404): ...` |
| Insufficient budget | `api error (402): ...` |
| Invalid WKT geometry | `api error (400): ...` |

The HTTP status code is always included in API errors.

---

## Development

### Run the hygiene check

Runs formatting, linting, and tests in one step:

```bash
./scripts/hygiene.sh
```

Equivalent individual steps:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

### Tests

Unit tests live in companion `_test.rs` files adjacent to each source module (e.g.,
`orders_test.rs` next to `orders.rs`). Contract tests in `src/openapi_contract_tests.rs`
verify request serialization and mock-response deserialization against the checked-in
`openapi.json` schema. The hygiene ratchet in `tests/hygiene.rs` enforces zero-budget bans
on `unwrap()`, `expect()`, `panic!()`, `unreachable!()`, `todo!()`, and `unimplemented!()`
in production `src/` code.

### Project layout

```
src/
  main.rs                     # Entry point: arg parsing, config loading, command dispatch
  cli.rs                      # Clap CLI definitions for all commands and their arguments
  client.rs                   # HTTP client wrapper around reqwest
  client_test.rs              # Client-specific unit tests
  config.rs                   # Config file loading and saving (~/.config/skyfi/config.toml)
  config_test.rs              # Config unit tests
  error.rs                    # Unified CliError type
  openapi_contract_tests.rs   # Request/response contract tests against openapi.json
  output.rs                   # Human-readable and JSON output helpers
  output_test.rs              # Output formatting unit tests
  research.rs                 # Prompt-driven research engine and tool loop
  research_test.rs            # Research engine unit tests
  types.rs                    # Shared request/response types (serde models)
  commands/
    mod.rs                    # Re-exports command modules
    alerts.rs                 # Alerts polling, local state, and service installers
    archives.rs               # archives search, archives get
    config.rs                 # config show, set-key, set-url
    feasibility.rs            # feasibility check, status, pass-prediction
    notifications.rs          # notifications list, get, create, delete
    orders.rs                 # orders list, get, order-archive, order-tasking, pass-targeted,
                              #   download, redeliver
    research.rs               # CLI wrapper for the research engine
tests/
  hygiene.rs                  # Ratchet test for panic/unwrap-style hygiene regressions
scripts/
  hygiene.sh                  # Repo-native fmt/clippy/test maintenance script
```
