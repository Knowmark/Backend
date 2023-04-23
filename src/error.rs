use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigurationError {
    #[error("configuration file not found in '{0}'")]
    NotFound(PathBuf),
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum BackendError {
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),

    // External errors
    #[error(transparent)]
    Database(#[from] mongodb::error::Error),
    #[error(transparent)]
    Bson(#[from] bson::de::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error(transparent)]
    Decode(#[from] base64::DecodeError),
}
