use thiserror::Error;

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("Network error during request: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Internal error during request: {0}")]
    Internal(#[from] fred::error::Error),
}