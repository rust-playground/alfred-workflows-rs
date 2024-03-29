use crate::database::models::{InsertMonitor, InsertScreenBoard, InsertTimeBoard};
use crate::errors::Error;
use reqwest::blocking::Client;

const APPLICATION_KEY: &str = "application_key";
const API_KEY: &str = "api_key";

pub struct Api<'a> {
    key: &'a str,
    application_key: &'a str,
    url: &'a str,
    subdomain: &'a str,
    client: Client,
}

impl<'a> Api<'a> {
    #[inline]
    pub fn new(key: &'a str, application_key: &'a str, url: &'a str, subdomain: &'a str) -> Self {
        Self {
            key,
            application_key,
            url,
            subdomain,
            client: reqwest::blocking::Client::new(),
        }
    }

    #[inline]
    pub fn get_timeboards(&self) -> Result<Vec<InsertTimeBoard>, Error> {
        #[derive(Debug, Deserialize)]
        struct Dashboards {
            #[serde(rename = "dashes")]
            boards: Vec<InsertTimeBoard>,
        }
        let results = self
            .client
            .get(format!("{}/v1/dash", self.url))
            .query(&[(APPLICATION_KEY, self.application_key), (API_KEY, self.key)])
            .send()?
            .json::<Dashboards>()?
            .boards;
        Ok(results)
    }

    #[inline]
    pub fn get_screenboards(&self) -> Result<Vec<InsertScreenBoard>, Error> {
        #[derive(Debug, Deserialize)]
        struct ScreenBoards {
            #[serde(rename = "screenboards")]
            boards: Vec<InsertScreenBoard>,
        }

        let results = self
            .client
            .get(format!("{}/v1/screen", self.url))
            .query(&[(APPLICATION_KEY, self.application_key), (API_KEY, self.key)])
            .send()?
            .json::<ScreenBoards>()?
            .boards;
        Ok(results)
    }

    #[inline]
    pub fn get_monitors(&self) -> Result<Vec<InsertMonitor>, Error> {
        let results = self
            .client
            .get(format!("{}/v1/monitor", self.url))
            .query(&[(APPLICATION_KEY, self.application_key), (API_KEY, self.key)])
            .send()?
            .json::<Vec<InsertMonitor>>()?;
        Ok(results)
    }
}
