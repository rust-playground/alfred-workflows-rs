use crate::database::DbContext;
use crate::errors::Error;
use crate::github::GitHubAPI;
use alfred::Item;

pub struct Workflow<'a> {
    api_key: &'a str,
    db: DbContext,
}

impl<'a> Workflow<'a> {
    /// # Errors
    ///
    /// Will return `Err` if `database` could not be connected to.
    ///
    #[inline]
    pub fn new(api_key: &'a str, database_url: &str) -> Result<Self, Error> {
        let db = DbContext::new(database_url)?;
        Ok(Workflow { api_key, db })
    }

    /// # Errors
    ///
    /// Will return `Err` if Contacting GitHub returns an error.
    ///
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

    /// # Errors
    ///
    /// Will return `Err` if querying the database fails.
    ///
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
