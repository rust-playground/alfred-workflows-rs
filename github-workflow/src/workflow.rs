use crate::database::models::NewRepository;
use crate::database::DbContext;
use alfred::Item;
use chrono::prelude::*;
use failure::{format_err, Error};
use reqwest::header::CONTENT_TYPE;
use reqwest::Response;
use serde_json::Value;
use std::borrow::Borrow;

pub struct GithubWorkflow {
    db: DbContext,
}

impl GithubWorkflow {
    pub fn create() -> Result<Self, Error> {
        let db = DbContext::new()?;
        Ok(GithubWorkflow { db })
    }

    pub fn set_token(&self, token: &str) -> Result<(), Error> {
        self.db.run_migrations()?;
        self.db.set_token(token)?;
        Ok(())
    }

    pub fn refresh_cache(&mut self) -> Result<(), Error> {
        self.db.run_migrations()?;
        let gh_token = self.db.get_token()?.value;

        //        let mut g = Github::new(gh_token)
        //            .map_err(|e| format_err!("failed to initialize GitHub client: {}", e))?;

        self.db.delete_repositories()?;
        self.refresh(gh_token.as_ref(), "")?;
        // and DB cleanup work
        self.db.optimize()?;
        Ok(())
    }

    fn refresh(&mut self, token: &str, cursor: &str) -> Result<(), Error> {
        let arg = if cursor != "" {
            format!(", after:\"{}\"", cursor)
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
        let mut escaped = query.to_string();
        escaped = escaped.replace("\n", "\\n");
        escaped = escaped.replace("\"", "\\\"");

        let mut q = String::from("{ \"query\": \"");
        q.push_str(&escaped);
        q.push_str("\" }");

        if cursor != "" {
            println!("query:{}", &q);
        }

        let mut resp: Results = reqwest::Client::new()
            .post("https://api.github.com/graphql")
            .bearer_auth(token)
            .header(CONTENT_TYPE, "application/json")
            .body(q)
            .send()?
            .json()?;

        let repositories = results
            .data
            .viewer
            .repositories
            .edges
            .iter()
            .filter_map(|edge| match edge {
                Some(e) => e.node.as_ref(),
                _ => None,
            })
            .map(|node| NewRepository {
                name_with_owner: node.name_with_owner.as_ref(),
                name: node.name.as_ref(),
                url: node.url.as_ref(),
                pushed_at: node.pushed_at.naive_utc(),
            })
            .collect::<Vec<NewRepository>>();

        self.db.insert_repositories(&repositories)?;

        let r = results.data.viewer.repositories.page_info;
        if r.has_next_page {
            // TODO: split the github fetching into it's own module and make it non self referencing.
            return self.refresh(token, &r.end_cursor);
        }
        Ok(())
    }

    pub fn query<'items>(&self, repo_name: &str) -> Result<Vec<Item<'items>>, Error> {
        self.db
            .find_repositories(repo_name, 10)?
            .into_iter()
            .map(|repo| {
                Ok(alfred::ItemBuilder::new(repo.name_with_owner)
                    .subtitle(repo.name.clone())
                    .autocomplete(repo.name)
                    .arg(format!("open {}", repo.url))
                    .into_item())
            })
            .collect::<Result<Vec<_>, _>>()
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
