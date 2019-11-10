use snafu::Snafu;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Database error: {}", err))]
    Sqlite { err: String },
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::Sqlite {
            err: err.to_string(),
        }
    }
}
