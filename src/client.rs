//! HTTP client wrapper for the SkyFi Platform API.
//!
//! This module provides a thin wrapper around `reqwest::Client` that:
//! - Resolves and attaches the API key on every request
//! - Applies a configurable per-request timeout
//! - Converts non-2xx responses into typed `CliError::Api` values with the
//!   HTTP status code and the server's error message
//!
//! # Authentication
//!
//! The SkyFi API uses a proprietary `X-Skyfi-Api-Key` header (not Bearer
//! token / Authorization header). The key is resolved from the config file
//! first, then from the `SKYFI_API_KEY` environment variable, giving the
//! env var precedence for CI and secrets-manager workflows.
//!
//! # Error extraction
//!
//! When the API returns a non-2xx response, `check_status` tries to parse the
//! body as JSON and extract the `detail` field (the Platform API's standard
//! error shape). If that fails it falls back to the raw body text, and
//! finally to the canonical HTTP reason phrase.

use crate::config::Config;
use crate::error::CliError;
use reqwest::header::{HeaderMap, HeaderValue};
use std::time::Duration;

/// Thin wrapper around `reqwest::Client` that attaches authentication and
/// base-URL handling to every outbound request.
pub struct Client {
    http: reqwest::Client,
    base_url: String,
}

impl Client {
    /// Construct a new `Client` from the loaded config and a timeout in seconds.
    ///
    /// Returns `CliError::Config` if no API key is available in either the
    /// config file or the `SKYFI_API_KEY` environment variable.
    pub fn new(config: &Config, timeout: u64) -> Result<Self, CliError> {
        // WHY: env var takes precedence over the config file so that CI pipelines
        // can inject credentials without touching the on-disk config.
        let api_key = config
            .api
            .api_key
            .clone()
            .or_else(|| std::env::var("SKYFI_API_KEY").ok())
            .ok_or_else(|| {
                CliError::Config(
                    "no API key configured. Set via `skyfi config set-key <KEY>` or SKYFI_API_KEY env var".into(),
                )
            })?;

        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Skyfi-Api-Key",
            HeaderValue::from_str(&api_key)
                .map_err(|e| CliError::Config(format!("invalid API key header: {e}")))?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(timeout))
            .build()?;

        Ok(Self {
            http,
            base_url: config.api.base_url.clone(),
        })
    }

    /// Build a full URL by joining the configured base URL with a path fragment.
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Send a GET request and return the response on success, or a `CliError` on failure.
    pub async fn get(&self, path: &str) -> Result<reqwest::Response, CliError> {
        let resp = self.http.get(self.url(path)).send().await?;
        check_status(resp).await
    }

    /// Send a GET request with URL query parameters serialized from `query`.
    pub async fn get_query<T: serde::Serialize>(
        &self,
        path: &str,
        query: &T,
    ) -> Result<reqwest::Response, CliError> {
        let resp = self.http.get(self.url(path)).query(query).send().await?;
        check_status(resp).await
    }

    /// Send a POST request with a JSON body serialized from `body`.
    pub async fn post<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<reqwest::Response, CliError> {
        let resp = self.http.post(self.url(path)).json(body).send().await?;
        check_status(resp).await
    }

    /// Send a DELETE request.
    pub async fn delete(&self, path: &str) -> Result<reqwest::Response, CliError> {
        let resp = self.http.delete(self.url(path)).send().await?;
        check_status(resp).await
    }
}

/// Inspect the response status and either return the response for the caller
/// to deserialize, or convert the error body into a `CliError::Api`.
///
/// EDGE: The response body is consumed here on error paths. Callers must not
/// attempt to read the body after this function returns `Err`.
async fn check_status(resp: reqwest::Response) -> Result<reqwest::Response, CliError> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }

    let status_code = status.as_u16();
    let default_message = status
        .canonical_reason()
        .unwrap_or("request failed")
        .to_string();
    let body = match resp.text().await {
        Ok(body) => body,
        Err(error) => {
            return Err(CliError::Api {
                status: status_code,
                message: format!("failed to read error response body: {error}"),
            });
        }
    };

    // WHY: Try structured error extraction first so callers see the API's own
    // message rather than raw JSON.
    if let Some(message) = api_error_message(&body) {
        return Err(CliError::Api {
            status: status_code,
            message,
        });
    }

    Err(CliError::Api {
        status: status_code,
        message: if body.trim().is_empty() {
            default_message
        } else {
            body
        },
    })
}

/// Extract the `detail` string from the Platform API's standard error response body.
///
/// Returns `None` if the body is not valid JSON or does not contain a string `detail` field.
fn api_error_message(body: &str) -> Option<String> {
    let err = serde_json::from_str::<serde_json::Value>(body).ok()?;

    err.get("detail")
        .and_then(|detail| detail.as_str())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
#[path = "client_test.rs"]
mod tests;
