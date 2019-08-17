use crate::database::models::{InsertMonitor, InsertScreenBoard, InsertTimeBoard};
use failure::Error;
use reqwest::Client;

const APPLICATION_KEY: &str = "application_key";
const API_KEY: &str = "api_key";

#[derive(Debug)]
pub struct DatadogAPI<'a> {
    api_key: &'a str,
    application_key: &'a str,
    client: Client,
}

impl<'a> DatadogAPI<'a> {
    #[inline]
    pub fn new(api_key: &'a str, application_key: &'a str) -> Self {
        Self {
            api_key,
            application_key,
            client: reqwest::Client::new(),
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
            .get("https://api.datadoghq.com/api/v1/dash")
            .query(&[
                (APPLICATION_KEY, self.application_key),
                (API_KEY, self.api_key),
            ])
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
            .get("https://api.datadoghq.com/api/v1/screen")
            .query(&[
                (APPLICATION_KEY, self.application_key),
                (API_KEY, self.api_key),
            ])
            .send()?
            .json::<ScreenBoards>()?
            .boards;
        Ok(results)
    }

    #[inline]
    pub fn get_monitors(&self) -> Result<Vec<InsertMonitor>, Error> {
        let results = self
            .client
            .get("https://api.datadoghq.com/api/v1/monitor")
            .query(&[
                (APPLICATION_KEY, self.application_key),
                (API_KEY, self.api_key),
            ])
            .send()?
            .json::<Vec<InsertMonitor>>()?;
        Ok(results)
    }
}
