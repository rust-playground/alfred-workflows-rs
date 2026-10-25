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
        let conn = Connection::open(database_url)?;
        Ok(DbContext { conn })
    }

    #[inline]
    pub fn find_repositories(&self, repo_name: &str, limit: i64) -> Result<Vec<Repository>, Error> {
        // This will allow searching by full name or just the words within the name;
        // it's not a regex but it's good enough.
        let name = format!(
            "%{}%",
            repo_name
                .split_terminator(' ')
                .flat_map(|s| s.split_terminator('_'))
                .flat_map(|s| s.split_terminator('-'))
                .collect::<Vec<&str>>()
                .join("%")
        );

        self.conn.prepare(
            "SELECT name_with_owner, name, url, pushed_at FROM repositories WHERE name LIKE ? ORDER BY pushed_at DESC LIMIT ?",
        )?.query_map([&name as &dyn ToSql,&limit], |row| {
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

    /// Atomically replaces the cached repositories. The table is recreated so schema
    /// changes apply on refresh; repositories visible to multiple accounts are stored once.
    #[inline]
    pub fn replace_repositories(&mut self, repositories: &[Repository]) -> Result<(), Error> {
        let tx = self.conn.transaction()?;
        tx.execute_batch(
            "DROP TABLE IF EXISTS repositories;
            CREATE TABLE repositories (
                url             TEXT     NOT NULL PRIMARY KEY,
                name_with_owner TEXT     NOT NULL,
                name            TEXT     NOT NULL,
                pushed_at       DATETIME NOT NULL
            );",
        )?;
        let mut stmt = tx.prepare(
            "INSERT INTO repositories (url, name_with_owner, name, pushed_at) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(url) DO NOTHING",
        )?;

        for repo in repositories {
            stmt.execute([
                &repo.url as &dyn ToSql,
                &repo.name_with_owner,
                &repo.name,
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
        self.conn.execute("VACUUM;", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    fn repo(url: &str) -> Repository {
        let name = url.rsplit('/').next().unwrap().to_string();
        Repository {
            name_with_owner: format!("org/{name}"),
            name,
            url: url.to_string(),
            pushed_at: DateTime::<Utc>::UNIX_EPOCH,
        }
    }

    fn urls(db: &DbContext, name: &str) -> Vec<String> {
        db.find_repositories(name, 10)
            .unwrap()
            .into_iter()
            .map(|r| r.url)
            .collect()
    }

    #[test]
    fn duplicate_repository_across_accounts_is_stored_once() {
        let mut db = DbContext::new(":memory:").unwrap();
        db.replace_repositories(&[
            repo("https://github.com/org/shared"),
            repo("https://github.com/org/shared"),
        ])
        .unwrap();
        assert_eq!(urls(&db, "shared"), ["https://github.com/org/shared"]);
    }

    #[test]
    fn same_name_on_different_hosts_is_kept() {
        let mut db = DbContext::new(":memory:").unwrap();
        db.replace_repositories(&[
            repo("https://github.com/org/svc"),
            repo("https://ghe.corp.com/org/svc"),
        ])
        .unwrap();
        assert_eq!(urls(&db, "svc").len(), 2);
    }

    #[test]
    fn replace_drops_previous_repositories() {
        let mut db = DbContext::new(":memory:").unwrap();
        db.replace_repositories(&[repo("https://github.com/org/old")])
            .unwrap();
        db.replace_repositories(&[repo("https://github.com/org/new")])
            .unwrap();
        assert_eq!(urls(&db, "old"), Vec::<String>::new());
        assert_eq!(urls(&db, "new"), ["https://github.com/org/new"]);
    }
}
