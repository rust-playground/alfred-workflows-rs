pub mod errors;
pub mod models;
pub mod monitors;
pub mod screenboards;
pub mod timeboards;

use crate::database::errors::Error;
use crate::database::models::Dashboard;
use crate::database::monitors::Monitors;
use crate::database::screenboards::Screenboards;
use crate::database::timeboards::Timeboards;
use rusqlite::{Connection, ToSql, NO_PARAMS};

#[derive(Debug)]
pub struct DbContext {
    conn: Connection,
    subdomain: String,
}

impl DbContext {
    #[inline]
    pub fn new(database_url: &str, subdomain: String) -> Result<Self, Error> {
        let conn = Connection::open(&database_url)?;
        Ok(DbContext { conn, subdomain })
    }

    // TODO: make interior mutable instead of everything having to be mutable

    #[inline]
    pub fn monitors(&mut self) -> Monitors {
        Monitors::new(self)
    }

    #[inline]
    pub fn timeboards(&mut self) -> Timeboards {
        Timeboards::new(self)
    }

    #[inline]
    pub fn screenboards(&mut self) -> Screenboards {
        Screenboards::new(self)
    }

    #[inline]
    pub fn find_dashboard(&self, title: &str, limit: i64) -> Result<Vec<Dashboard>, Error> {
        // This will allow searching by full name or just the words within the name;
        // it's not a regex but it's good enough.
        let query = format!(
            "%{}%",
            title
                .split_terminator(' ')
                .flat_map(|s| s.split_terminator('_'))
                .flat_map(|s| s.split_terminator('-'))
                .collect::<Vec<&str>>()
                .join("%")
        );

        self.conn.prepare(
            "SELECT title, description, url, modified FROM timeboards WHERE title LIKE ?1
                         UNION ALL
                         SELECT title, description, url, modified FROM screenboards WHERE title LIKE ?1
                         ORDER BY modified DESC
                         LIMIT ?",
        )?.query_map(&[&query as &dyn ToSql,&limit], |row| {
            Ok(Dashboard{
                title:row.get(0)?,
                description:row.get(1)?,
                url: row.get(2)?,
            })
        })?.map(|r|{
            Ok(r?)
        }).collect::<Result<Vec<_>, _>>()
    }

    #[inline]
    pub fn run_migrations(&mut self) -> Result<(), Error> {
        self.timeboards().run_migrations()?;
        self.screenboards().run_migrations()?;
        self.monitors().run_migrations()?;
        Ok(())
    }

    #[inline]
    pub fn optimize(&self) -> Result<(), Error> {
        // since this workflow is READ heavy, let's optimize the SQLite indexes and DB
        self.conn.execute("VACUUM;", NO_PARAMS)?;
        Ok(())
    }
}
