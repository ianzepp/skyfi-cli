# skyfi-cli

Rust CLI for the SkyFi Platform API.

This project wraps the SkyFi Platform API v2 command surface for common operational flows:

- connectivity and auth checks
- archive search and retrieval
- archive and tasking orders
- notification management
- feasibility checks and pass prediction
- pricing lookups

## Install

### Homebrew (macOS and Linux)

```bash
brew install ianzepp/tap/skyfi-cli
```

### Shell script

Downloads the latest release binary for your platform:

```bash
curl -fsSL https://raw.githubusercontent.com/ianzepp/skyfi-cli/master/install.sh | bash
```

### Build from source

```bash
cargo install --path .
```

Or run directly without installing:

```bash
cargo run -- --help
```

## Configuration

By default, configuration lives at `~/.config/skyfi/config.toml`.

Set an API key once:

```bash
skyfi-cli config set-key <YOUR_KEY>
```

Or use an environment variable:

```bash
export SKYFI_API_KEY=<YOUR_KEY>
```

Inspect current config:

```bash
skyfi-cli config show
```

`config show` redacts the stored API key instead of echoing secrets back to the terminal.

Override the API base URL:

```bash
skyfi-cli config set-url https://app.skyfi.com/platform-api
```

The URL is validated before it is saved.

## Common Commands

Verify connectivity:

```bash
skyfi-cli ping
skyfi-cli whoami
```

Search archives:

```bash
skyfi-cli archives search \
  --aoi 'POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))'
```

Inspect a specific archive:

```bash
skyfi-cli archives get <ARCHIVE_ID>
```

Create an archive order:

```bash
skyfi-cli orders order-archive \
  --aoi 'POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))' \
  --archive-id <ARCHIVE_ID>
```

Run a feasibility check:

```bash
skyfi-cli feasibility check \
  --aoi 'POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))' \
  --product-type day \
  --resolution HIGH \
  --start-date 2025-04-01 \
  --end-date 2025-04-15
```

To block until the feasibility job finishes:

```bash
skyfi-cli feasibility check \
  --aoi 'POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))' \
  --product-type day \
  --resolution HIGH \
  --start-date 2025-04-01 \
  --end-date 2025-04-15 \
  --wait
```

Create a pass-targeted tasking order:

```bash
skyfi-cli orders pass-targeted \
  --aoi 'POLYGON ((-122.4 37.7, -122.3 37.7, -122.3 37.8, -122.4 37.8, -122.4 37.7))' \
  --window-start 2025-04-01T00:00:00Z \
  --window-end 2025-04-15T00:00:00Z \
  --product-type day \
  --resolution HIGH
```

This predicts matching passes, picks the earliest one by default, and places a tasking
order pinned to that `providerWindowId`. To inspect candidate passes yourself first, use:

```bash
skyfi-cli feasibility pass-prediction --aoi 'POINT (-122.4 37.7)' --from-date 2025-04-01 --to-date 2025-04-07
```

All `--aoi` values are WKT strings such as `POLYGON ((...))` or `POINT (...)`.

Get machine-readable JSON from any command:

```bash
skyfi-cli --json orders list
```

## Validation

```bash
./scripts/hygiene.sh
```

Equivalent manual steps:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
