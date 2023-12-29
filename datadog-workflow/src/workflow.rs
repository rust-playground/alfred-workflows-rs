use crate::database::DbContext;
use crate::datadog::Api;
use crate::errors::Error;
use alfred::Item;
use std::str;

pub struct Workflow<'a> {
    api_key: &'a str,
    application_key: &'a str,
    api_url: &'a str,
    db: DbContext,
    subdomain: &'a str,
}

impl<'a> Workflow<'a> {
    /// Creates a new `DataDog` workflow for use.
    ///
    /// # Errors
    /// Can return when database error occurs.
    #[inline]
    pub fn new(
        api_key: &'a str,
        application_key: &'a str,
        database_url: &str,
        api_url: &'a str,
        subdomain: &'a str,
    ) -> Result<Self, Error> {
        let db = DbContext::new(database_url, subdomain.to_owned())?;
        Ok(Workflow {
            api_key,
            application_key,
            api_url,
            db,
            subdomain,
        })
    }

    /// Refreshes cached data from `DataDog` API
    ///
    /// # Errors
    /// can return when database error occurs or API.
    pub fn refresh_cache(&mut self) -> Result<(), Error> {
        let datadog_api = Api::new(
            self.api_key,
            self.application_key,
            self.api_url,
            self.subdomain,
        );
        self.db.run_migrations()?;
        self.refresh_timeboards(&datadog_api)?;
        self.refresh_screenboards(&datadog_api)?;
        self.refresh_monitors(&datadog_api)?;

        // and DB cleanup work
        Ok(self.db.optimize()?)
    }

    fn refresh_timeboards(&mut self, datadog_api: &Api) -> Result<(), Error> {
        let mut db = self.db.timeboards();
        db.delete_all()?;
        let results = datadog_api.get_timeboards()?;
        db.insert(&results)?;
        Ok(())
    }

    fn refresh_screenboards(&mut self, datadog_api: &Api) -> Result<(), Error> {
        let mut db = self.db.screenboards();
        db.delete_all()?;
        let results = datadog_api.get_screenboards()?;
        db.insert(&results)?;
        Ok(())
    }

    fn refresh_monitors(&mut self, datadog_api: &Api) -> Result<(), Error> {
        let mut db = self.db.monitors();
        db.delete_all()?;
        let results = datadog_api.get_monitors()?;
        db.insert(&results)?;
        Ok(())
    }

    /// Query `DataDog` Time Boards
    ///
    /// # Errors
    /// can return when database error occurs.
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

    /// Query `DataDog` Screen Boards
    ///
    /// # Errors
    /// can return when database error occurs.
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

    /// Query `DataDog` Dashboards
    ///
    /// # Errors
    /// can return when database error occurs.
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

    /// Query `DataDog` Monitors
    ///
    /// # Errors
    /// can return when database error occurs.
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
