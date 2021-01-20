
use thiserror::Error;
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP error: {}", _0)]
    HTTP(String),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
}
