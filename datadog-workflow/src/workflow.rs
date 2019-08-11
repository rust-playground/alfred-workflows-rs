use crate::database::models::{InsertMonitor, InsertScreenBoard, InsertTimeBoard};
use crate::database::DbContext;
use alfred::Item;
use failure::Error;
use reqwest::Client;
use serde::Deserialize;
use std::str;

const APPLICATION_KEY: &str = "application_key";
const API_KEY: &str = "api_key";

pub struct DatadogWorkflow<'a> {
    api_key: &'a str,
    application_key: &'a str,
    db: DbContext,
}

impl<'a> DatadogWorkflow<'a> {
    #[inline]
    pub fn new(
        api_key: &'a str,
        application_key: &'a str,
        database_url: &str,
    ) -> Result<Self, Error> {
        let db = DbContext::new(database_url)?;
        Ok(DatadogWorkflow {
            api_key,
            application_key,
            db,
        })
    }

    pub fn refresh_cache(&mut self) -> Result<(), Error> {
        let client = reqwest::Client::new();
        self.db.run_migrations()?;
        self.refresh_timeboards(&client)?;
        self.refresh_screenboards(&client)?;
        self.refresh_monitors(&client)?;

        // and DB cleanup work
        self.db.optimize()
    }

    fn refresh_timeboards(&mut self, client: &Client) -> Result<(), Error> {
        let mut db = self.db.timeboards();
        db.delete_all()?;

        #[derive(Debug, Deserialize)]
        struct Dashboards {
            #[serde(rename = "dashes")]
            boards: Vec<InsertTimeBoard>,
        }
        let results = client
            .get("https://api.datadoghq.com/api/v1/dash")
            .query(&[
                (APPLICATION_KEY, self.application_key),
                (API_KEY, self.api_key),
            ])
            .send()?
            .json::<Dashboards>()?
            .boards;
        db.insert(&results)?;
        Ok(())
    }

    fn refresh_screenboards(&mut self, client: &Client) -> Result<(), Error> {
        let mut db = self.db.screenboards();
        db.delete_all()?;

        #[derive(Debug, Deserialize)]
        struct ScreenBoards {
            #[serde(rename = "screenboards")]
            boards: Vec<InsertScreenBoard>,
        }

        let results = client
            .get("https://api.datadoghq.com/api/v1/screen")
            .query(&[
                (APPLICATION_KEY, self.application_key),
                (API_KEY, self.api_key),
            ])
            .send()?
            .json::<ScreenBoards>()?
            .boards;
        db.insert(&results)?;
        Ok(())
    }

    fn refresh_monitors(&mut self, client: &Client) -> Result<(), Error> {
        let mut db = self.db.monitors();
        db.delete_all()?;

        let results = client
            .get("https://api.datadoghq.com/api/v1/monitor")
            .query(&[
                (APPLICATION_KEY, self.application_key),
                (API_KEY, self.api_key),
            ])
            .send()?
            .json::<Vec<InsertMonitor>>()?;
        db.insert(&results)?;
        Ok(())
    }

    pub fn query_timeboards<'items>(&mut self, title: &str) -> Result<Vec<Item<'items>>, Error> {
        let results = self.db.timeboards().find(title, 10)?;
        let items = results
            .into_iter()
            .map(|m| {
                alfred::ItemBuilder::new(m.title.clone())
                    .subtitle(m.description)
                    .autocomplete(m.title)
                    .arg(format!("open {}", m.url))
                    .into_item()
            })
            .collect();
        Ok(items)
    }

    pub fn query_screenboards<'items>(&mut self, title: &str) -> Result<Vec<Item<'items>>, Error> {
        let results = self.db.screenboards().find(title, 10)?;
        let items = results
            .into_iter()
            .map(|m| {
                alfred::ItemBuilder::new(m.title.clone())
                    .subtitle(m.description)
                    .autocomplete(m.title)
                    .arg(format!("open {}", m.url))
                    .into_item()
            })
            .collect();
        Ok(items)
    }

    pub fn query_dashboards<'items>(&self, title: &str) -> Result<Vec<Item<'items>>, Error> {
        let results = self.db.find_dashboard(title, 10)?;
        let items = results
            .into_iter()
            .map(|m| {
                alfred::ItemBuilder::new(m.title.clone())
                    .subtitle(m.description)
                    .autocomplete(m.title)
                    .arg(format!("open {}", m.url))
                    .into_item()
            })
            .collect();
        Ok(items)
    }

    pub fn query_monitors<'items>(
        &mut self,
        name: &str,
        tag: Option<&str>,
    ) -> Result<Vec<Item<'items>>, Error> {
        let results = self.db.monitors().find(name, tag, 10)?;
        let items = results
            .into_iter()
            .map(|m| {
                alfred::ItemBuilder::new(m.name.clone())
                    .subtitle(m.name.clone())
                    .autocomplete(m.name)
                    .arg(format!("open {}", m.url))
                    .into_item()
            })
            .collect();
        Ok(items)
    }
}
