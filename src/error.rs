#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("{0}")]
    General(String),

    #[error("config: {0}")]
    Config(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("http: {0}")]
    Http(#[from] reqwest::Error),

    #[error("api error ({status}): {message}")]
    Api { status: u16, message: String },
}
