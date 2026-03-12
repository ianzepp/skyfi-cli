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
    let body = resp.text().await.unwrap_or_default();

    // Try to parse as API error
    if let Ok(err) = serde_json::from_str::<serde_json::Value>(&body) {
        let message = err
            .get("detail")
            .and_then(|d| d.as_str())
            .unwrap_or(&body)
            .to_string();
        return Err(CliError::Api {
            status: status_code,
            message,
        });
    }

    Err(CliError::Api {
        status: status_code,
        message: body,
    })
}
