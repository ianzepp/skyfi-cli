# skyfi-cli

Rust CLI for the SkyFi Platform API.

This project wraps the SkyFi Platform API v2 command surface for common operational flows:

- connectivity and auth checks
- archive search and retrieval
- archive and tasking orders
- notification management
- feasibility checks and pass prediction
- pricing lookups

## Status

The crate currently builds cleanly with `cargo build`, passes `cargo clippy --all-targets --all-features -- -D warnings`, and has a small unit-test baseline with `cargo test`.

## Build

```bash
cargo build
```

Run the CLI from source:

```bash
cargo run -- --help
```

Install it locally with Cargo:

```bash
cargo install --path .
```

Note: the compiled binary name is currently `skyfi-cli`, so installed commands use that executable name unless you rename the package or add an explicit binary target.

## Configuration

By default, configuration lives at `~/.config/skyfi/config.toml`.

Set an API key once:

```bash
cargo run -- config set-key <YOUR_KEY>
```

Or use an environment variable:

```bash
export SKYFI_API_KEY=<YOUR_KEY>
```

Inspect current config:

```bash
cargo run -- config show
```

`config show` redacts the stored API key instead of echoing secrets back to the terminal.

Override the API base URL:

```bash
cargo run -- config set-url https://app.skyfi.com/platform-api
```

The URL is validated before it is saved.

## Common Commands

Verify connectivity:

```bash
cargo run -- ping
cargo run -- whoami
```

Search archives:

```bash
cargo run -- archives search \
  --aoi '{"type":"Polygon","coordinates":[[[-122.4,37.7],[-122.3,37.7],[-122.3,37.8],[-122.4,37.8],[-122.4,37.7]]]}'
```

Inspect a specific archive:

```bash
cargo run -- archives get <ARCHIVE_ID>
```

Create an archive order:

```bash
cargo run -- orders order-archive \
  --aoi '{"type":"Polygon","coordinates":[[[-122.4,37.7],[-122.3,37.7],[-122.3,37.8],[-122.4,37.8],[-122.4,37.7]]]}' \
  --archive-id <ARCHIVE_ID>
```

Run a feasibility check:

```bash
cargo run -- feasibility check \
  --aoi '{"type":"Polygon","coordinates":[[[-122.4,37.7],[-122.3,37.7],[-122.3,37.8],[-122.4,37.8],[-122.4,37.7]]]}' \
  --product-type day \
  --resolution HIGH \
  --start-date 2025-04-01 \
  --end-date 2025-04-15
```

Get machine-readable JSON from any command:

```bash
cargo run -- --json orders list
```

## Validation

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
