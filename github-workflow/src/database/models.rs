use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct Repository {
    pub name_with_owner: String,
    pub name: String,
    pub url: String,
    pub pushed_at: DateTime<Utc>,
}
