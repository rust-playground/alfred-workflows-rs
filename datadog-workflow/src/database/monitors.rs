use crate::database::models::{InsertMonitor, Monitor};
use crate::database::DbContext;
use failure::{format_err, Error};
use rusqlite::{ToSql, NO_PARAMS};

pub struct Monitors<'a> {
    db: &'a mut DbContext,
}

impl<'a> Monitors<'a> {
    #[inline]
    pub fn new(db: &'a mut DbContext) -> Self {
        Self { db }
    }

    #[inline]
    pub fn run_migrations(&self) -> Result<(), Error> {
        self.db.conn.execute(
            "CREATE TABLE IF NOT EXISTS monitors (
                id          INTEGER  NOT NULL PRIMARY KEY,
                name        TEXT     NOT NULL,
                url         TEXT     NOT NULL,
                modified    DATETIME NOT NULL
            );",
            NO_PARAMS,
        )?;
        self.db.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_monitors_name_modified ON monitors (name, modified);",
            NO_PARAMS,
        )?;
        self.db.conn.execute(
            "CREATE TABLE IF NOT EXISTS monitor_tags (
                id          INTEGER NOT NULL,
                name        TEXT    NOT NULL,
                CONSTRAINT fk_monitors
                FOREIGN KEY (id)
                REFERENCES monitors(id)
                ON DELETE CASCADE
            );",
            NO_PARAMS,
        )?;
        self.db.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_monitor_tags_id ON monitor_tags (id);",
            NO_PARAMS,
        )?;
        self.db.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_monitor_tags_name ON monitor_tags (name);",
            NO_PARAMS,
        )?;
        Ok(())
    }

    #[inline]
    pub fn delete_all(&self) -> Result<(), Error> {
        self.db.conn.execute("DELETE FROM monitors;", NO_PARAMS)?;
        Ok(())
    }

    #[inline]
    pub fn insert(&mut self, monitors: &[InsertMonitor]) -> Result<(), Error> {
        let tx = self.db.conn.transaction()?;
        let mut stmt_monitor =
            tx.prepare("INSERT INTO monitors (id, name, url, modified) VALUES (?1, ?2, ?3, ?4)")?;
        let mut stmt_tags = tx.prepare("INSERT INTO monitor_tags (id, name) VALUES (?1, ?2)")?;

        for monitor in monitors {
            let url = format!("https://segment.datadoghq.com/monitors/{}", monitor.id);
            stmt_monitor.execute(&[
                &monitor.id as &dyn ToSql,
                &monitor.name,
                &url,
                &monitor.modified,
            ])?;
            for tag in &monitor.tags {
                stmt_tags.execute(&[&monitor.id as &dyn ToSql, &tag])?;
            }
        }

        stmt_monitor.finalize()?;
        stmt_tags.finalize()?;
        tx.commit()?;
        Ok(())
    }

    #[inline]
    pub fn find(&self, name: &str, tag: Option<&str>, limit: i64) -> Result<Vec<Monitor>, Error> {
        // This will allow searching by full name or just the words within the name;
        // it's not a regex but it's good enough.
        let query = format!(
            "%{}%",
            name.split_terminator(' ')
                .flat_map(|s| s.split_terminator('_'))
                .flat_map(|s| s.split_terminator('-'))
                .collect::<Vec<&str>>()
                .join("%")
        );

        let tag_query: String;
        let mut params: Vec<&dyn ToSql> = vec![&query];
        let mut select = "SELECT m.id, m.name, m.url, m.modified FROM monitors m ".to_owned();
        match tag {
            Some(ref t) => {
                select += "LEFT JOIN monitor_tags t ON t.id = m.id WHERE m.name LIKE ? AND t.name LIKE ? ";
                tag_query = format!(
                    "%{}%",
                    t.split_terminator(' ')
                        .flat_map(|s| s.split_terminator('_'))
                        .flat_map(|s| s.split_terminator('-'))
                        .collect::<Vec<&str>>()
                        .join("%")
                );
                params.push(&tag_query);
            }
            _ => select += "WHERE m.name LIKE ? ",
        }
        select += "ORDER BY m.modified DESC LIMIT ?";
        params.push(&limit);

        self.db
            .conn
            .prepare(&select)?
            .query_map(&params, |row| {
                Ok(Monitor {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    url: row.get(2)?,
                    modified: row.get(3)?,
                })
            })?
            .map(|r| match r {
                Ok(v) => Ok(v),
                Err(e) => Err(format_err!("Query + Transform into Monitor failed: {}", e)),
            })
            .collect::<Result<Vec<_>, _>>()
    }
}
