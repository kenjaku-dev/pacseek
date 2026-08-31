use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("config error: {0}")]
    Config(String),
    #[error("alpm error: {0}")]
    Alpm(String),
    #[error("aur error: {0}")]
    Aur(String),
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("regex error: {0}")]
    Regex(#[from] regex::Error),
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}
