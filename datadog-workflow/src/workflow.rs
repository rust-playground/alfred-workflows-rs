use alfred::Item;
use chrono::{DateTime, Utc};
use failure::{format_err, Error};
use reqwest::Client;
use rusqlite::{types::ToSql, Connection, NO_PARAMS};
use serde::Deserialize;
use std::str;

const APPLICATION_KEY: &str = "application_key";
const API_KEY: &str = "api_key";

pub struct DatadogWorkflow {
    conn: Connection,
}

impl DatadogWorkflow {
    pub fn create() -> Result<Self, Error> {
        let conn =
            alfred_workflow::open_database_or_else("datadog", DatadogWorkflow::create_tables)?;
        Ok(DatadogWorkflow { conn })
    }

    fn create_tables(conn: &Connection) -> Result<(), Error> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS config (
                key   TEXT NOT NULL PRIMARY KEY,
                value TEXT NOT NULL
            );",
            NO_PARAMS,
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS timeboards (
                id          TEXT    NOT NULL PRIMARY KEY,
                title       TEXT    NOT NULL,
                description TEXT    NOT NULL,
                url         TEXT    NOT NULL,
                modified    INTEGER NOT NULL
            );",
            NO_PARAMS,
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timeboards_title_modified ON timeboards (title, modified);",
            NO_PARAMS,
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS screenboards (
                id          INTEGER NOT NULL PRIMARY KEY,
                title       TEXT    NOT NULL,
                description TEXT    NOT NULL,
                url         TEXT    NOT NULL,
                modified    INTEGER NOT NULL
            );",
            NO_PARAMS,
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_screenboards_title_modified ON screenboards (title, modified);",
            NO_PARAMS,
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS monitors (
                id          INTEGER NOT NULL PRIMARY KEY,
                name        TEXT    NOT NULL,
                url         TEXT    NOT NULL,
                modified    INTEGER NOT NULL
            );",
            NO_PARAMS,
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_monitors_name_modified ON monitors (name, modified);",
            NO_PARAMS,
        )?;
        conn.execute(
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
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_monitor_tags_id ON monitor_tags (id);",
            NO_PARAMS,
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_monitor_tags_name ON monitor_tags (name);",
            NO_PARAMS,
        )?;
        Ok(())
    }

    pub fn set_application_key(&self, key: &str) -> Result<(), Error> {
        self.set_key(APPLICATION_KEY, key)
    }

    pub fn set_api_key(&self, key: &str) -> Result<(), Error> {
        self.set_key(API_KEY, key)
    }

    fn set_key(&self, name: &str, key: &str) -> Result<(), Error> {
        self.conn
            .execute(
                "INSERT INTO config (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                &[name, key],
            )
            .map(|_|Ok(()))
            .map_err(|e| format_err!("failed to insert application key: {}", e))?
    }

    pub fn refresh_cache(&mut self) -> Result<(), Error> {
        let mut stmt = self.conn.prepare("SELECT value FROM config WHERE key=?1")?;
        let application_key: String = stmt.query_row(&[APPLICATION_KEY], |row| row.get(0))?;
        let api_key: String = stmt.query_row(&[API_KEY], |row| row.get(0))?;
        stmt.finalize()?;

        let client = reqwest::Client::new();

        self.refresh_timeboards(&client, &application_key, &api_key)?;
        self.refresh_screenboards(&client, &application_key, &api_key)?;
        self.refresh_monitors(&client, &application_key, &api_key)?;

        // since this workflow is READ heavy, let's optimize the SQLite indexes and DB
        self.conn
            .execute("VACUUM;", NO_PARAMS)
            .map(|_| Ok(()))
            .map_err(|e| format_err!("failed to VACCUM database: {}", e))?
    }

    fn refresh_timeboards(
        &mut self,
        client: &Client,
        app_key: &str,
        api_key: &str,
    ) -> Result<(), Error> {
        self.conn
            .execute("DELETE FROM timeboards;", NO_PARAMS)
            .map_err(|e| format_err!("failed to delete timeboards: {}", e))?;

        #[derive(Debug, Deserialize)]
        struct Dashboards {
            #[serde(rename = "dashes")]
            boards: Vec<Dashboard>,
        }

        #[derive(Debug, Deserialize)]
        struct Dashboard {
            id: String,
            title: String,
            description: Option<String>,
            modified: DateTime<Utc>,
        }

        let tx = self.conn.transaction()?;
        let mut stmt = tx.prepare("INSERT INTO timeboards (id, title, description, url, modified) VALUES (?1, ?2, ?3, ?4, ?5)")?;

        for board in client
            .get("https://api.datadoghq.com/api/v1/dash")
            .query(&[(APPLICATION_KEY, app_key), (API_KEY, api_key)])
            .send()?
            .json::<Dashboards>()?
            .boards
        {
            let url = format!("https://segment.datadoghq.com/dash/{}", board.id);
            stmt.execute(&[
                &board.id as &ToSql,
                &board.title,
                &board.description.unwrap_or_default(),
                &url,
                &board.modified.timestamp(),
            ])?;
        }
        stmt.finalize()?;
        tx.commit()
            .map_err(|e| format_err!("failed to commit timeboards transaction: {}", e))?;
        Ok(())
    }

    fn refresh_screenboards(
        &mut self,
        client: &Client,
        app_key: &str,
        api_key: &str,
    ) -> Result<(), Error> {
        self.conn
            .execute("DELETE FROM screenboards;", NO_PARAMS)
            .map_err(|e| format_err!("failed to delete screenboards: {}", e))?;

        #[derive(Debug, Deserialize)]
        struct ScreenBoards {
            #[serde(rename = "screenboards")]
            boards: Vec<ScreenBoard>,
        }

        #[derive(Debug, Deserialize)]
        struct ScreenBoard {
            id: i32,
            title: String,
            description: Option<String>,
            modified: DateTime<Utc>,
        }

        let tx = self.conn.transaction()?;
        let mut stmt = tx.prepare("INSERT INTO screenboards (id, title, description, url, modified) VALUES (?1, ?2, ?3, ?4, ?5)")?;

        for board in client
            .get("https://api.datadoghq.com/api/v1/screen")
            .query(&[(APPLICATION_KEY, app_key), (API_KEY, api_key)])
            .send()?
            .json::<ScreenBoards>()?
            .boards
        {
            let url = format!("https://segment.datadoghq.com/screen/{}", board.id);
            stmt.execute(&[
                &board.id as &ToSql,
                &board.title,
                &board.description.unwrap_or_default(),
                &url,
                &board.modified.timestamp(),
            ])?;
        }
        stmt.finalize()?;
        tx.commit()
            .map_err(|e| format_err!("failed to commit  screenboards transaction: {}", e))?;
        Ok(())
    }

    fn refresh_monitors(
        &mut self,
        client: &Client,
        app_key: &str,
        api_key: &str,
    ) -> Result<(), Error> {
        self.conn
            .execute("DELETE FROM monitors;", NO_PARAMS)
            .map_err(|e| format_err!("failed to delete monitors: {}", e))?;

        #[derive(Debug, Deserialize)]
        struct Monitor {
            id: i32,
            name: String,
            tags: Vec<String>,
            modified: DateTime<Utc>,
        }

        let tx = self.conn.transaction()?;
        let mut stmt_monitor =
            tx.prepare("INSERT INTO monitors (id, name, url, modified) VALUES (?1, ?2, ?3, ?4)")?;
        let mut stmt_tags = tx.prepare("INSERT INTO monitor_tags (id, name) VALUES (?1, ?2)")?;

        for monitor in client
            .get("https://api.datadoghq.com/api/v1/monitor")
            .query(&[(APPLICATION_KEY, app_key), (API_KEY, api_key)])
            .send()?
            .json::<Vec<Monitor>>()?
        {
            let url = format!("https://segment.datadoghq.com/monitors/{}", monitor.id);
            stmt_monitor.execute(&[
                &monitor.id as &ToSql,
                &monitor.name,
                &url,
                &monitor.modified.timestamp(),
            ])?;
            for tag in monitor.tags {
                stmt_tags.execute(&[&monitor.id as &ToSql, &tag])?;
            }
        }
        stmt_monitor.finalize()?;
        stmt_tags.finalize()?;
        tx.commit()
            .map_err(|e| format_err!("failed to commit  screenboards transaction: {}", e))?;
        Ok(())
    }

    pub fn query_timeboards<'items>(&self, title: &str) -> Result<Vec<Item<'items>>, Error> {
        let query = format!("%{}%", title);
        self.conn.prepare(
            "SELECT title, description, url FROM timeboards WHERE title LIKE ? ORDER BY modified DESC LIMIT 10",
        )?.query_map(&[&query], |row| {
            let title: String = row.get(0);
            let description: String = row.get(1);
            let url: String = row.get(2);
            alfred::ItemBuilder::new(title.clone())
                .subtitle(description)
                .autocomplete(title)
                .arg(format!("open {}", url))
                .into_item()
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format_err!("failed querying timeboards: {}", e))
    }

    pub fn query_screenboards<'items>(&self, title: &str) -> Result<Vec<Item<'items>>, Error> {
        let query = format!("%{}%", title);
        self.conn.prepare(
            "SELECT title, description, url FROM screenboards WHERE title LIKE ? ORDER BY modified DESC LIMIT 10",
        )?.query_map(&[&query], |row| {
            let title: String = row.get(0);
            let description: String = row.get(1);
            let url: String = row.get(2);
            alfred::ItemBuilder::new(title.clone())
                .subtitle(description)
                .autocomplete(title)
                .arg(format!("open {}", url))
                .into_item()
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format_err!("failed querying screenboards: {}", e))
    }

    pub fn query_dashboards<'items>(&self, title: &str) -> Result<Vec<Item<'items>>, Error> {
        let query = format!("%{}%", title);
        self.conn
            .prepare(
                "SELECT title, description, url, modified FROM timeboards WHERE title LIKE ?1
                 UNION ALL
                 SELECT title, description, url, modified FROM screenboards WHERE title LIKE ?1
                 ORDER BY modified
                 LIMIT 10",
            )?
            .query_map(&[&query], |row| {
                let title: String = row.get(0);
                let description: String = row.get(1);
                let url: String = row.get(2);
                alfred::ItemBuilder::new(title.clone())
                    .subtitle(description)
                    .autocomplete(title)
                    .arg(format!("open {}", url))
                    .into_item()
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format_err!("failed querying dashboards: {}", e))
    }

    pub fn query_monitors<'items>(
        &self,
        name: &str,
        tag: Option<&str>,
    ) -> Result<Vec<Item<'items>>, Error> {
        let query = format!("%{}%", name);
        let tag_query: String;
        let mut params: Vec<&ToSql> = vec![&query];
        let mut select = "SELECT m.name, m.url FROM monitors m ".to_owned();
        match tag {
            Some(ref t) => {
                select += "LEFT JOIN monitor_tags t ON t.id = m.id WHERE m.name LIKE ? AND t.name LIKE ? ";
                tag_query = format!("{}%", t);
                params.push(&tag_query);
            }
            _ => select += "WHERE m.name LIKE ? ",
        }
        select += "ORDER BY m.modified DESC LIMIT 10";

        self.conn
            .prepare(&select)?
            .query_map(&params, |row| {
                let name: String = row.get(0);
                let url: String = row.get(1);
                alfred::ItemBuilder::new(name.clone())
                    .subtitle(name.clone())
                    .autocomplete(name)
                    .arg(format!("open {}", url))
                    .into_item()
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format_err!("failed querying monitors: {}", e))
    }
}
