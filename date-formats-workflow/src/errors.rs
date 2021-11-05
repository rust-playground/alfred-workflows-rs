use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{}", _0)]
    Text(String),

    #[error("failed to write alfred items->json {}", _0)]
    WriteItems(#[from] io::Error),

    #[error(transparent)]
    Parse(#[from] anyhow::Error),
}
