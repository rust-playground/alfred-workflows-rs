#[macro_use]
extern crate serde;

#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

pub(crate) mod database;
pub(crate) mod github;
pub mod workflow;
