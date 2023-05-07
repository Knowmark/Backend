use thiserror::Error;

#[derive(Debug, Error)]
pub enum BackendError {
    // External errors
    #[error(transparent)]
    Database(#[from] mongodb::error::Error),
    #[error(transparent)]
    Bson(#[from] bson::de::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Jwt(#[from] jsonwebtoken::errors::Error),
}
