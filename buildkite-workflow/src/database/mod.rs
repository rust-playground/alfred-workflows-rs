pub mod errors;
pub mod models;

use crate::database::models::Pipeline;
use errors::Result;
use rusqlite::{Connection, ToSql, NO_PARAMS};

pub struct DbContext {
    conn: Connection,
}

impl DbContext {
    #[inline]
    pub fn new(database_url: &str) -> Result<Self> {
        let conn = Connection::open(&database_url)?;
        Ok(DbContext { conn })
    }

    #[inline]
    pub fn run_migrations(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS pipelines (
                unique_name TEXT    NOT NULL PRIMARY KEY,
                name        TEXT    NOT NULL,
                url         TEXT    NOT NULL
            );",
        )?;
        Ok(())
    }

    #[inline]
    pub fn delete_pipelines(&self) -> Result<()> {
        self.conn.execute("DELETE FROM pipelines;", NO_PARAMS)?;
        Ok(())
    }

    #[inline]
    pub fn find_pipelines(&self, repo_name: &[String], limit: i64) -> Result<Vec<Pipeline>> {
        // This will allow searching by full name or just the words within the name;
        // it's not a regex but it's good enough.
        let query = format!(
            "%{}%",
            repo_name
                .iter()
                .flat_map(|s| s.split_terminator(' '))
                .flat_map(|s| s.split_terminator('_'))
                .flat_map(|s| s.split_terminator('-'))
                .collect::<Vec<&str>>()
                .join("%")
        );

        let results = self.conn.prepare(
            "SELECT unique_name, name, url FROM pipelines WHERE name LIKE ? ORDER BY name ASC LIMIT ?",
        )?.query_map(&[&query as &dyn ToSql,&limit], |row| {
            Ok(Pipeline{
                unique_name: row.get(0)?,
                name:row.get(1)?,
                url:row.get(2)?,
            })
        })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(results)
    }

    #[inline]
    pub fn insert_pipelines(&mut self, pipelines: &[Pipeline]) -> Result<()> {
        let tx = self.conn.transaction()?;
        let mut stmt =
            tx.prepare("INSERT INTO pipelines (unique_name, name, url) VALUES (?1, ?2, ?3)")?;

        for pipeline in pipelines {
            stmt.execute(&[
                &pipeline.unique_name as &dyn ToSql,
                &pipeline.name,
                &pipeline.url,
            ])?;
        }

        stmt.finalize()?;
        tx.commit()?;
        Ok(())
    }

    #[inline]
    pub fn optimize(&self) -> Result<()> {
        // since this workflow is READ heavy, let's optimize the SQLite indexes and DB
        self.conn.execute("VACUUM;", NO_PARAMS)?;
        Ok(())
    }
}
