pub mod errors;
pub mod models;

use crate::database::models::Repository;
use errors::Error;
use rusqlite::{Connection, ToSql};

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
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS repositories (
                    name_with_owner TEXT     NOT NULL PRIMARY KEY,
                    name            TEXT     NOT NULL,
                    url             TEXT     NOT NULL,
                    pushed_at       DATETIME NOT NULL
                );",
        )?;
        // CREATE VIRTUAL TABLE IF NOT EXISTS repositories_fts4 using fts4(content)
        Ok(())
    }

    #[inline]
    pub fn delete_repositories(&self) -> Result<(), Error> {
        self.conn.execute_batch(
            "DELETE FROM repositories;
                ",
        )?;
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

        // "SELECT name_with_owner, name, url, pushed_at FROM repositories LEFT JOIN(SELECT content FROM repositories_fts4 WHERE content MATCH ?) ON content = name_with_owner ORDER BY pushed_at DESC LIMIT ?",

        self.conn.prepare(
            "SELECT name_with_owner, name, url, pushed_at FROM repositories WHERE name LIKE ? ORDER BY pushed_at DESC LIMIT ?",
        )?.query_map(&[&query as &dyn ToSql,&limit], |row| {
            Ok(Repository{
                name_with_owner: row.get(0)?,
                name:row.get(1)?,
                url:row.get(2)?,
                pushed_at:row.get(3)?,
            })
        })?.map(|r|{
            Ok(r?)
        }).collect::<Result<Vec<_>, _>>()
    }

    #[inline]
    pub fn insert_repositories(&mut self, repositories: &[Repository]) -> Result<(), Error> {
        let tx = self.conn.transaction()?;
        let mut stmt = tx.prepare("INSERT INTO repositories (name_with_owner, name, url, pushed_at) VALUES (?1, ?2, ?3, ?4)")?;

        for repo in repositories {
            stmt.execute(&[
                &repo.name_with_owner as &dyn ToSql,
                &repo.name,
                &repo.url,
                &repo.pushed_at,
            ])?;
        }

        stmt.finalize()?;
        tx.commit()?;

        // let tx = self.conn.transaction()?;
        // let mut stmt2 = tx.prepare("INSERT INTO repositories_fts4 (content) VALUES (?1)")?;

        // for repo in repositories {
        //     stmt2.execute(&[&repo.name_with_owner as &dyn ToSql])?;
        // }

        // stmt2.finalize()?;
        // tx.commit()?;

        Ok(())
    }

    #[inline]
    pub fn optimize(&self) -> Result<(), Error> {
        // since this workflow is READ heavy, let's optimize the SQLite indexes and DB
        self.conn.execute("VACUUM;", [])?;
        Ok(())
    }
}
