use snafu::Snafu;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Database error: {}", err))]
    HTTP { err: String },

    #[snafu(display("Reqwest error: {}", err))]
    ReqwestError { err: reqwest::Error },
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::ReqwestError { err: err }
    }
}
