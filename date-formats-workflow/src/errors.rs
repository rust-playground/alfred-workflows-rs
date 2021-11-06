use std::io;
use std::num::ParseIntError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{}", _0)]
    Text(String),

    #[error("failed to write alfred items->json {}", _0)]
    WriteItems(#[from] io::Error),

    #[error("failed to parse integer {}", _0)]
    ParseInt(#[from] ParseIntError),

    #[error("failed to parse DateTime")]
    ParseDateTime,

    #[error("failed to parse DateTime from unix timestamp")]
    UnixTimestamp,
}
