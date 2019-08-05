use crate::database::models::NewRepository;
use crate::database::DbContext;
use crate::github::GitHubAPI;
use alfred::Item;
use chrono::prelude::*;
use failure::Error;

pub struct GithubWorkflow {
    db: DbContext,
}

impl GithubWorkflow {
    pub fn create() -> Result<Self, Error> {
        let db = DbContext::new()?;
        Ok(GithubWorkflow { db })
    }

    pub fn set_token(&self, token: &str) -> Result<(), Error> {
        self.db.run_migrations()?;
        self.db.set_token(token)?;
        Ok(())
    }

    pub fn refresh_cache(&mut self) -> Result<(), Error> {
        self.db.run_migrations()?;
        let gh_token = self.db.get_token()?.value;
        let api = GitHubAPI::new(gh_token.as_str());

        self.db.delete_repositories()?;

        for v in api.accessible_repositories() {
            self.db.insert_repositories(
                v.iter()
                    .map(|r| NewRepository {
                        name_with_owner: r.name_with_owner.as_ref(),
                        name: r.name.as_ref(),
                        url: r.url.as_ref(),
                        pushed_at: r.pushed_at,
                    })
                    .collect::<Vec<NewRepository>>()
                    .as_ref(),
            )?;
        }

        // and DB cleanup work
        self.db.optimize()?;
        Ok(())
    }

    pub fn query<'items>(&self, repo_name: &str) -> Result<Vec<Item<'items>>, Error> {
        self.db
            .find_repositories(repo_name, 10)?
            .into_iter()
            .map(|repo| {
                Ok(alfred::ItemBuilder::new(repo.name_with_owner)
                    .subtitle(repo.name.clone())
                    .autocomplete(repo.name)
                    .arg(format!("open {}", repo.url))
                    .into_item())
            })
            .collect::<Result<Vec<_>, _>>()
    }
}
