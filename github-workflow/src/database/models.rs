use crate::database::schema::{config, repositories};
use chrono::NaiveDateTime;

#[derive(Debug, Queryable)]
pub struct Repository {
    pub name_with_owner: String,
    pub name: String,
    pub url: String,
    pub pushed_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[table_name = "repositories"]
pub struct NewRepository<'a> {
    pub name_with_owner: &'a str,
    pub name: &'a str,
    pub url: &'a str,
    pub pushed_at: NaiveDateTime,
}

#[derive(Debug, Queryable)]
pub struct Config {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Insertable)]
#[table_name = "config"]
pub struct NewConfig<'a> {
    pub key: &'a str,
    pub value: &'a str,
}
