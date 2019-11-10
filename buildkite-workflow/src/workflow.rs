use crate::buildkite_api::BuildkiteAPI;
use crate::database::models::Pipeline;
use crate::database::DbContext;
use alfred::Item;
use std::error::Error;

pub struct BuildkiteWorkflow<'a> {
    api_key: &'a str,
    db: DbContext,
}

impl<'a> BuildkiteWorkflow<'a> {
    #[inline]
    pub fn new(api_key: &'a str, database_url: &str) -> Result<Self, Box<dyn Error>> {
        let db = DbContext::new(database_url)?;
        Ok(BuildkiteWorkflow { api_key, db })
    }

    #[inline]
    pub fn refresh_cache(&mut self) -> Result<(), Box<dyn Error>> {
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

    #[inline]
    pub fn query<'items>(&self, repo_name: &[String]) -> Result<Vec<Item<'items>>, Box<dyn Error>> {
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
