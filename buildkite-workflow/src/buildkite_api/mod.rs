pub mod errors;
pub mod models;

use crate::buildkite_api::models::{Organization, Pipeline};
use errors::{Error, Result};
use regex::Regex;
use reqwest::header::{HeaderValue, CONTENT_TYPE, LINK};

#[derive(Debug)]
pub struct BuildkiteAPI<'a> {
    token: &'a str,
    re: Regex,
}

impl<'a> BuildkiteAPI<'a> {
    #[inline]
    pub fn new(token: &'a str) -> Self {
        let re = Regex::new(r#"<(.*)>; rel="next""#).unwrap();
        Self { token, re }
    }

    #[inline]
    pub fn get_organizations_paginated(&self) -> OrganizationsIter {
        OrganizationsIter {
            api: self,
            next: Some("https://api.buildkite.com/v2/organizations?per_page=100".to_owned()),
        }
    }

    #[inline]
    pub fn get_pipelines_paginated(&self, organization: &str) -> PipelinesIter {
        PipelinesIter {
            api: self,
            next: Some(format!(
                "https://api.buildkite.com/v2/organizations/{}/pipelines?per_page=100",
                organization
            )),
        }
    }

    #[inline]
    fn fetch_organizations(&self, url: &str) -> Result<OrganizationResponse> {
        let response = reqwest::blocking::Client::new()
            .get(url)
            .bearer_auth(self.token)
            .header(CONTENT_TYPE, "application/json")
            .send()?;

        if !response.status().is_success() {
            return Err(Error::Http(response.text()?));
        }

        let link = response.headers().get(LINK);
        let next = self.extract_next(link);
        let results: Vec<Organization> = response.json()?;
        Ok(OrganizationResponse { next, results })
    }

    #[inline]
    fn fetch_pipelines(&self, url: &str) -> Result<PipelineResponse> {
        let response = reqwest::blocking::Client::new()
            .get(url)
            .bearer_auth(self.token)
            .header(CONTENT_TYPE, "application/json")
            .send()?;

        if !response.status().is_success() {
            return Err(Error::Http(response.text()?));
        }

        let link = response.headers().get(LINK);
        let next = self.extract_next(link);
        let results: Vec<Pipeline> = response.json()?;
        Ok(PipelineResponse { next, results })
    }

    #[inline]
    fn extract_next(&self, link: Option<&HeaderValue>) -> Option<String> {
        link.and_then(|h| self.re.captures(h.to_str().unwrap()))
            .and_then(|cap| cap.get(1))
            .map(|c| c.as_str().to_owned())
    }
}

struct OrganizationResponse {
    next: Option<String>,
    results: Vec<Organization>,
}

pub struct OrganizationsIter<'a> {
    api: &'a BuildkiteAPI<'a>,
    next: Option<String>,
}

impl<'a> Iterator for OrganizationsIter<'a> {
    type Item = Result<Vec<Organization>>;

    fn next(&mut self) -> Option<Self::Item> {
        let response = self.api.fetch_organizations(self.next.as_ref()?);
        if response.is_err() {
            return Some(Err(response.err().unwrap()));
        }
        let response = response.unwrap();
        self.next = response.next;
        Some(Ok(response.results))
    }
}

struct PipelineResponse {
    next: Option<String>,
    results: Vec<Pipeline>,
}

pub struct PipelinesIter<'a> {
    api: &'a BuildkiteAPI<'a>,
    next: Option<String>,
}

impl<'a> Iterator for PipelinesIter<'a> {
    type Item = Result<Vec<Pipeline>>;

    fn next(&mut self) -> Option<Self::Item> {
        let response = self.api.fetch_pipelines(self.next.as_ref()?);
        if response.is_err() {
            return Some(Err(response.err().unwrap()));
        }
        let response = response.unwrap();
        self.next = response.next;
        Some(Ok(response.results))
    }
}
