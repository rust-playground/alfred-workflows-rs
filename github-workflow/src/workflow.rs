use alfred::Item;
use chrono::prelude::*;
use failure::{format_err, Error};
use github_gql::{client::Github, query::Query};
use rusqlite::{types::ToSql, Connection, NO_PARAMS};

pub struct GithubWorkflow {
    conn: Connection,
}

impl GithubWorkflow {
    pub fn create() -> Result<Self, Error> {
        let conn = alfred_workflow::open_database_or_else("github", GithubWorkflow::create_tables)?;
        Ok(GithubWorkflow { conn })
    }

    fn create_tables(conn: &Connection) -> Result<(), Error> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS repositories (
                    name_with_owner TEXT NOT NULL PRIMARY KEY,
                    name            TEXT NOT NULL,
                    url             TEXT NOT NULL,
                    pushed_at       INTEGER NOT NULL
                );",
            NO_PARAMS,
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS config (
                    key   TEXT NOT NULL PRIMARY KEY,
                    value TEXT NOT NULL
                );",
            NO_PARAMS,
        )?;
        Ok(())
    }

    pub fn set_token(&self, token: &str) -> Result<(), Error> {
        self.conn
            .execute(
                "INSERT INTO config (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                &["token", token],
            )
            .map(|_|Ok(()))
            .map_err(|e| format_err!("failed to insert token: {}", e))?
    }

    pub fn refresh_cache(&mut self) -> Result<(), Error> {
        let gh_token: String = self
            .conn
            .prepare("SELECT value FROM config WHERE key=?")?
            .query_row(&["token"], |row| row.get(0))?;

        let mut g = Github::new(gh_token)
            .map_err(|e| format_err!("failed to initialize GitHub client: {}", e))?;

        self.conn
            .execute("DELETE FROM repositories;", NO_PARAMS)
            .map_err(|e| format_err!("failed to delete existing repositories: {}", e))?;

        self.refresh(&mut g, "")?;

        // since this workflow is READ heavy, let's optimize the SQLite indexes and DB
        self.conn
            .execute("VACUUM;", NO_PARAMS)
            .map(|_| Ok(()))
            .map_err(|e| format_err!("failed to VACCUM database: {}", e))?
    }

    fn refresh(&mut self, g: &mut Github, cursor: &str) -> Result<(), Error> {
        let arg = if cursor != "" {
            format!(", after:\\\"{}\\\"", cursor)
        } else {
            "".to_string()
        };

        let query = format!(
            "query {{ \
                viewer {{ \
                    repositories(first: 100, affiliations: [OWNER, COLLABORATOR, ORGANIZATION_MEMBER], ownerAffiliations: [OWNER, COLLABORATOR, ORGANIZATION_MEMBER]{}) {{ \
                        pageInfo {{ \
                            hasNextPage \
                            endCursor \
                        }} \
                        edges {{ \
                            node {{ \
                                name \
                                nameWithOwner \
                                pushedAt \
                                url \
                            }} \
                        }} \
                    }} \
                }} \
            }}",
            arg
        );
        let (_, _, res) = g
            .query::<Results>(&Query::new_raw(query))
            .map_err(|e| format_err!("failed GitHub query, check permissions: {}", e))?;

        if let Some(res) = res {
            let tx = self.conn.transaction()?;
            let mut stmt = tx.prepare("INSERT INTO repositories (name_with_owner, name, url, pushed_at) VALUES (?1, ?2, ?3, ?4)")?;

            let nodes =
                res.data
                    .viewer
                    .repositories
                    .edges
                    .into_iter()
                    .filter_map(|edge| match edge {
                        Some(e) => e.node,
                        _ => None,
                    });
            for node in nodes {
                stmt.execute(&[
                    &node.name_with_owner as &ToSql,
                    &node.name,
                    &node.url,
                    &node.pushed_at.timestamp(),
                ])
                .map_err(|e| format_err!("failed to insert repository record: {}", e))?;
            }

            stmt.finalize()?;
            tx.commit()
                .map_err(|e| format_err!("failed to commit repositories transaction: {}", e))?;

            let r = res.data.viewer.repositories.page_info;
            if r.has_next_page {
                return self.refresh(g, &r.end_cursor);
            }
        }
        Ok(())
    }

    pub fn query<'items>(&self, repo_name: &str) -> Result<Vec<Item<'items>>, Error> {
        let query = format!("%{}%", repo_name);
        self.conn.prepare(
            "SELECT name_with_owner, name, url FROM repositories WHERE name LIKE ? ORDER BY pushed_at DESC LIMIT 10",
        )?.query_map(&[&query], |row| {
            let name_with_owner: String =      row.get(0)?;
            let name: String = row.get(1)?;
            let url: String = row.get(2)?;
            Ok(alfred::ItemBuilder::new(name_with_owner)
                .subtitle(name.clone())
                .autocomplete(name)
                .arg(format!("open {}", url))
                .into_item())
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format_err!("failed querying items: {}", e))
    }
}

#[derive(Debug, Deserialize)]
struct PageInfo {
    #[serde(rename = "endCursor")]
    end_cursor: String,
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
}

#[derive(Debug, Deserialize)]
struct Node {
    name: String,
    #[serde(rename = "nameWithOwner")]
    name_with_owner: String,
    url: String,
    #[serde(rename = "pushedAt")]
    pushed_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct Edge {
    node: Option<Node>,
}

#[derive(Debug, Deserialize)]
struct Repositories {
    edges: Vec<Option<Edge>>,
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
struct Viewer {
    repositories: Repositories,
}

#[derive(Debug, Deserialize)]
struct Data {
    viewer: Viewer,
}

#[derive(Debug, Deserialize)]
struct Results {
    data: Data,
}
