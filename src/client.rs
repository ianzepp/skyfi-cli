use crate::config::Config;
use crate::error::CliError;
use reqwest::header::{HeaderMap, HeaderValue};
use std::time::Duration;

pub struct Client {
    http: reqwest::Client,
    base_url: String,
}

impl Client {
    pub fn new(config: &Config, timeout: u64) -> Result<Self, CliError> {
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

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub async fn get(&self, path: &str) -> Result<reqwest::Response, CliError> {
        let resp = self.http.get(self.url(path)).send().await?;
        check_status(resp).await
    }

    pub async fn get_query<T: serde::Serialize>(
        &self,
        path: &str,
        query: &T,
    ) -> Result<reqwest::Response, CliError> {
        let resp = self.http.get(self.url(path)).query(query).send().await?;
        check_status(resp).await
    }

    pub async fn post<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<reqwest::Response, CliError> {
        let resp = self.http.post(self.url(path)).json(body).send().await?;
        check_status(resp).await
    }

    pub async fn delete(&self, path: &str) -> Result<reqwest::Response, CliError> {
        let resp = self.http.delete(self.url(path)).send().await?;
        check_status(resp).await
    }
}

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

fn api_error_message(body: &str) -> Option<String> {
    let err = serde_json::from_str::<serde_json::Value>(body).ok()?;

    err.get("detail")
        .and_then(|detail| detail.as_str())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
#[path = "client_test.rs"]
mod tests;
