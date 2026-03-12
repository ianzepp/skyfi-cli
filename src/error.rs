//! Unified error type for all CLI failure modes.
//!
//! Every fallible operation in the CLI returns `CliError`. The `thiserror` derive
//! macro generates `Display` implementations, and `#[from]` conversions let the
//! `?` operator propagate errors from reqwest, serde_json, toml, and std::io
//! without manual wrapping at each call site.
//!
//! API errors carry the HTTP status code alongside the server message so the
//! caller can make policy decisions (e.g., exit code, retry logic) based on
//! whether the failure was a 401, 404, 429, or 5xx.

/// All errors that can arise during a CLI command.
///
/// Variants cover the full path from config file I/O through HTTP transport
/// and API-level failures. The `Api` variant is populated by `client::check_status`,
/// which extracts the `detail` field from the JSON error body when present.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// A general, human-readable error message not covered by a more specific variant.
    #[error("{0}")]
    General(String),

    /// Configuration error: missing key, invalid URL, parse failure, etc.
    #[error("config: {0}")]
    Config(String),

    /// Filesystem I/O error (reading/writing config file, etc.).
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization failure.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    /// HTTP transport error from reqwest (connection refused, timeout, TLS, etc.).
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),

    /// TOML serialization error when writing the config file.
    #[error("serialization: {0}")]
    Serialize(#[from] toml::ser::Error),

    /// The API returned a non-2xx HTTP status.
    ///
    /// `status` is the raw HTTP status code. `message` is sourced from the
    /// response body's `detail` field when present, falling back to the raw
    /// body text, and finally to the canonical HTTP reason phrase.
    #[error("api error ({status}): {message}")]
    Api { status: u16, message: String },
}
