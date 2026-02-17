use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum FrostCliError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Frost API error: {message} (reason: {reason})")]
    FrostApi { reason: String, message: String },

    #[error("Databricks SQL error: {0}")]
    Databricks(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Environment variable missing: {0}")]
    EnvVar(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
}

impl From<dotenvy::Error> for FrostCliError {
    fn from(e: dotenvy::Error) -> Self {
        FrostCliError::EnvVar(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, FrostCliError>;

impl FrostCliError {
    pub fn frost_api(reason: impl Into<String>, message: impl Into<String>) -> Self {
        FrostCliError::FrostApi {
            reason: reason.into(),
            message: message.into(),
        }
    }

    pub fn databricks(message: impl fmt::Display) -> Self {
        FrostCliError::Databricks(message.to_string())
    }

    pub fn config(message: impl fmt::Display) -> Self {
        FrostCliError::Config(message.to_string())
    }
}
