use crate::database::DbContext;
use crate::github::GitHubAPI;
use alfred::Item;
use failure::Error;

pub struct GithubWorkflow<'a> {
    api_key: &'a str,
    db: DbContext,
}

impl<'a> GithubWorkflow<'a> {
    #[inline]
    pub fn create(api_key: &'a str, database_url: &str) -> Result<Self, Error> {
        let db = DbContext::new(database_url)?;
        Ok(GithubWorkflow { api_key, db })
    }

    #[inline]
    pub fn refresh_cache(&mut self) -> Result<(), Error> {
        self.db.run_migrations()?;
        let api = GitHubAPI::new(self.api_key);

        self.db.delete_repositories()?;

        for v in api.accessible_repositories() {
            self.db.insert_repositories(&v)?;
        }
        // and DB cleanup work
        self.db.optimize()?;
        Ok(())
    }

    #[inline]
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
