use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    SQLite(#[from] rusqlite::Error),
}
