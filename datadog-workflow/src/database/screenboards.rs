use crate::database::models::{InsertScreenBoard, ScreenBoard};
use crate::database::DbContext;
use failure::{format_err, Error};
use rusqlite::{ToSql, NO_PARAMS};

pub struct Screenboards<'a> {
    db: &'a mut DbContext,
}

impl<'a> Screenboards<'a> {
    #[inline]
    pub fn new(db: &'a mut DbContext) -> Self {
        Self { db }
    }

    #[inline]
    pub fn run_migrations(&self) -> Result<(), Error> {
        self.db.conn.execute(
            "CREATE TABLE IF NOT EXISTS screenboards (
                id          INTEGER  NOT NULL PRIMARY KEY,
                title       TEXT     NOT NULL,
                description TEXT     NOT NULL,
                url         TEXT     NOT NULL,
                modified    DATETIME NOT NULL
            );",
            NO_PARAMS,
        )?;
        self.db.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_screenboards_title_modified ON screenboards (title, modified);",
            NO_PARAMS,
        )?;
        Ok(())
    }

    #[inline]
    pub fn delete_all(&self) -> Result<(), Error> {
        self.db
            .conn
            .execute("DELETE FROM screenboards;", NO_PARAMS)?;
        Ok(())
    }

    #[inline]
    pub fn insert(&mut self, screenboards: &[InsertScreenBoard]) -> Result<(), Error> {
        let tx = self.db.conn.transaction()?;
        let mut stmt = tx.prepare("INSERT INTO screenboards (id, title, description, url, modified) VALUES (?1, ?2, ?3, ?4, ?5)")?;

        for board in screenboards {
            let url = format!("https://segment.datadoghq.com/screen/{}", board.id);
            stmt.execute(&[
                &board.id as &ToSql,
                &board.title,
                &board.description.clone().unwrap_or_default(),
                &url,
                &board.modified.timestamp(),
            ])?;
        }

        stmt.finalize()?;
        tx.commit()?;
        Ok(())
    }

    #[inline]
    pub fn find(&self, title: &str, limit: i64) -> Result<Vec<ScreenBoard>, Error> {
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

        self.db.conn.prepare(
            "SELECT id, title, description, url, modified FROM screenboards WHERE title LIKE ? ORDER BY modified DESC LIMIT ?",
        )?.query_map(&[&query as &ToSql,&limit], |row| {
            Ok(ScreenBoard{
                id: row.get(0)?,
                title:row.get(1)?,
                description:row.get(2)?,
                url: row.get(3)?,
                modified:row.get(4)?,
            })
        })?.map(|r|{
            match r{
                Ok(v) => Ok(v),
                Err(e)=> Err(format_err!("Query + Transform into ScreenBoard failed: {}",e)),
            }
        }).collect::<Result<Vec<_>, _>>()
    }
}
