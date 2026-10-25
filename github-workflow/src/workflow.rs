use crate::database::DbContext;
use crate::errors::Error;
use crate::github::{self, Account, GitHubAPI};
use alfred::Item;

pub struct Workflow {
    accounts: Vec<Account>,
    db: DbContext,
}

impl Workflow {
    /// `api_keys` is a comma-separated list of `<token>` or `<token>@<ghes-host>` entries.
    ///
    /// # Errors
    ///
    /// Will return `Err` if `api_keys` is malformed or `database` could not be connected to.
    ///
    #[inline]
    pub fn new(api_keys: &str, database_url: &str) -> Result<Self, Error> {
        let accounts = github::parse_accounts(api_keys)?;
        let db = DbContext::new(database_url)?;
        Ok(Workflow { accounts, db })
    }

    /// Fetches repositories for every account before touching the cache, so a failing
    /// account leaves the existing cache intact.
    ///
    /// # Errors
    ///
    /// Will return `Err` if Contacting GitHub returns an error.
    ///
    #[inline]
    pub fn refresh_cache(&mut self) -> Result<(), Error> {
        let mut repositories = Vec::new();
        for account in &self.accounts {
            let api = GitHubAPI::new(account);
            for page in api.accessible_repositories() {
                repositories.extend(page?);
            }
        }
        self.db.replace_repositories(&repositories)?;
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
