use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid API_KEY entry: {0}")]
    InvalidAccount(&'static str),

    #[error("GitHub request to {host} failed")]
    Request {
        host: String,
        #[source]
        source: reqwest::Error,
    },

    #[error(transparent)]
    SQLite(#[from] crate::database::errors::Error),
}
