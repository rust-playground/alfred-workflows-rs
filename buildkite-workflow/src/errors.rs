use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    API(#[from] crate::buildkite_api::errors::Error),

    #[error(transparent)]
    SQLite(#[from] crate::database::errors::Error),

    #[error("failed to write alfred items->json {}", _0)]
    WriteItems(#[from] io::Error),
}
