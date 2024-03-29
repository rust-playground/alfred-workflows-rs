use crate::buildkite_api::BuildkiteAPI;
use crate::database::models::Pipeline;
use crate::database::DbContext;
use crate::errors::Error;
use alfred::Item;

pub struct Workflow<'a> {
    api_key: &'a str,
    db: DbContext,
}

impl<'a> Workflow<'a> {
    /// Create a new Workflow
    ///
    /// # Errors
    ///
    /// Will return `Err` if database connection fails.
    ///
    #[inline]
    pub fn new(api_key: &'a str, database_url: &str) -> Result<Self, Error> {
        let db = DbContext::new(database_url)?;
        Ok(Workflow { api_key, db })
    }

    /// Refreshes DB with all Buildkite information.
    ///
    /// # Errors
    ///
    /// Will return `Err` if database connection fails or hitting the `Buildkite` API fails
    ///
    #[inline]
    pub fn refresh_cache(&mut self) -> Result<(), Error> {
        self.db.run_migrations()?;
        let api = BuildkiteAPI::new(self.api_key);
        self.db.delete_pipelines()?;
        for organizations in api.get_organizations_paginated() {
            for org in organizations? {
                for pipelines in api.get_pipelines_paginated(&org.slug) {
                    let pl = pipelines?
                        .into_iter()
                        .map(|p| Pipeline {
                            url: format!("https://buildkite.com/{}/{}", &org.slug, &p.name),
                            unique_name: format!("{}/{}", &org.slug, &p.name),
                            name: p.name,
                        })
                        .collect::<Vec<Pipeline>>();
                    self.db.insert_pipelines(&pl)?;
                }
            }
        }
        // and DB cleanup work
        self.db.optimize()?;
        Ok(())
    }

    /// Queries the stored information using the given query.
    ///
    /// # Errors
    ///
    /// Will return `Err` if database connection fails.
    ///
    #[inline]
    pub fn query<'items>(&self, repo_name: &[String]) -> Result<Vec<Item<'items>>, Error> {
        self.db
            .find_pipelines(repo_name, 10)?
            .into_iter()
            .map(|repo| {
                Ok(alfred::ItemBuilder::new(repo.unique_name)
                    .subtitle(repo.name.clone())
                    .autocomplete(repo.name)
                    .arg(format!("open {}", repo.url))
                    .into_item())
            })
            .collect::<Result<Vec<_>, _>>()
    }
}
