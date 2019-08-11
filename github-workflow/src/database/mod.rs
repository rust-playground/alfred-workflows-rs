pub mod models;

use crate::database::models::Repository;
use failure::{format_err, Error};
use rusqlite::{Connection, ToSql, NO_PARAMS};

pub struct DbContext {
    conn: Connection,
}

impl DbContext {
    #[inline]
    pub fn new(database_url: &str) -> Result<Self, Error> {
        let conn = Connection::open(&database_url)?;
        Ok(DbContext { conn })
    }

    #[inline]
    pub fn run_migrations(&self) -> Result<(), Error> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS repositories (
                    name_with_owner TEXT     NOT NULL PRIMARY KEY,
                    name            TEXT     NOT NULL,
                    url             TEXT     NOT NULL,
                    pushed_at       DATETIME NOT NULL
                );",
            NO_PARAMS,
        )?;
        Ok(())
    }

    #[inline]
    pub fn delete_repositories(&self) -> Result<(), Error> {
        self.conn.execute("DELETE FROM repositories;", NO_PARAMS)?;
        Ok(())
    }

    #[inline]
    pub fn find_repositories(&self, repo_name: &str, limit: i64) -> Result<Vec<Repository>, Error> {
        // This will allow searching by full name or just the words within the name;
        // it's not a regex but it's good enough.
        let query = format!(
            "%{}%",
            repo_name
                .split_terminator(' ')
                .flat_map(|s| s.split_terminator('_'))
                .flat_map(|s| s.split_terminator('-'))
                .collect::<Vec<&str>>()
                .join("%")
        );

        let results = self.conn.prepare(
            "SELECT name_with_owner, name, url, pushed_at FROM repositories WHERE name LIKE ? ORDER BY pushed_at DESC LIMIT ?",
        )?.query_map(&[&query as &ToSql,&limit], |row| {
            Ok(Repository{
                name_with_owner: row.get(0)?,
                name:row.get(1)?,
                url:row.get(2)?,
                pushed_at:row.get(3)?,
            })
        })?.map(|r|{
            match r{
                Ok(v) => Ok(v),
                Err(e)=> Err(format_err!("Query + Transform into Repository failed: {}",e)),
            }
        }).collect::<Result<Vec<_>, _>>();
        results
    }

    #[inline]
    pub fn insert_repositories(&mut self, repositories: &[Repository]) -> Result<(), Error> {
        let tx = self.conn.transaction()?;
        let mut stmt = tx.prepare("INSERT INTO repositories (name_with_owner, name, url, pushed_at) VALUES (?1, ?2, ?3, ?4)")?;

        for repo in repositories {
            stmt.execute(&[
                &repo.name_with_owner as &ToSql,
                &repo.name,
                &repo.url,
                &repo.pushed_at,
            ])?;
        }

        stmt.finalize()?;
        tx.commit()?;
        Ok(())
    }

    #[inline]
    pub fn optimize(&self) -> Result<(), Error> {
        // since this workflow is READ heavy, let's optimize the SQLite indexes and DB
        self.conn.execute("VACUUM;", NO_PARAMS)?;
        Ok(())
    }
}
