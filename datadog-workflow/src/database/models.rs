use chrono::{DateTime, Utc};

#[derive(Debug, Deserialize)]
pub struct InsertMonitor {
    pub id: i32,
    pub name: String,
    pub tags: Vec<String>,
    pub modified: DateTime<Utc>,
}

#[derive(Debug)]
pub struct Monitor {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub modified: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct InsertTimeBoard {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub modified: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct TimeBoard {
    pub id: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub modified: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct InsertScreenBoard {
    pub id: i32,
    pub title: String,
    pub description: Option<String>,
    pub modified: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ScreenBoard {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub url: String,
    pub modified: DateTime<Utc>,
}

#[derive(Debug)]
pub struct Dashboard {
    pub title: String,
    pub description: String,
    pub url: String,
}
