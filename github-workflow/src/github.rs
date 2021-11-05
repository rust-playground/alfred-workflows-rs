use crate::database::models::Repository;
use crate::errors::Error;
use chrono::{DateTime, Utc};
use reqwest::header::{CONTENT_TYPE, USER_AGENT};

#[derive(Debug)]
pub struct GitHubAPI<'a> {
    token: &'a str,
}

impl<'a> GitHubAPI<'a> {
    #[inline]
    pub const fn new(token: &'a str) -> Self {
        Self { token }
    }

    #[inline]
    pub const fn accessible_repositories(&self) -> OwnedRepositories {
        OwnedRepositories {
            api: self,
            has_more: true,
            cursor: None,
        }
    }

    #[inline]
    fn fetch_repositories(&self, cursor: Option<String>) -> Result<Results, Error> {
        let arg = cursor.map_or_else(|| String::from(""), |v| format!(", after:\"{}\"", v));
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
                                pushedAt \
                                url \
                            }} \
                        }} \
                    }} \
                }} \
            }}",
            arg
        );

        // TODO: clean this up with a proper type that will escape automatically when serialized to JSON
        let mut escaped = query;
        escaped = escaped.replace("\n", "\\n");
        escaped = escaped.replace("\"", "\\\"");

        let mut q = String::from("{ \"query\": \"");
        q.push_str(&escaped);
        q.push_str("\" }");

        let results: Results = reqwest::blocking::Client::new()
            .post("https://api.github.com/graphql")
            .bearer_auth(self.token)
            .header(CONTENT_TYPE, "application/json")
            .header(USER_AGENT, "Alfred Github Workflow")
            .body(q)
            .send()?
            .json()?;

        Ok(results)
    }
}

pub struct OwnedRepositories<'a> {
    api: &'a GitHubAPI<'a>,
    has_more: bool,
    cursor: Option<String>,
}

impl<'a> Iterator for OwnedRepositories<'a> {
    type Item = Vec<Repository>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.has_more {
            return None;
        }
        let results = self
            .api
            .fetch_repositories(self.cursor.take())
            .expect("unable to fetch data from the GitHub API");
        self.has_more = results.data.viewer.repositories.page_info.has_next_page;
        if self.has_more {
            self.cursor = Some(results.data.viewer.repositories.page_info.end_cursor)
        }
        Some(
            results
                .data
                .viewer
                .repositories
                .edges
                .into_iter()
                .filter_map(|edge| match edge {
                    Some(e) => e.node,
                    _ => None,
                })
                .map(|node| {
                    let mut s = node.url.rsplit('/');
                    let name = s.next().unwrap_or_default().to_string();
                    let owner = s.next().unwrap_or_default();
                    Repository {
                        name_with_owner: format!("{}/{}", owner, name),
                        name,
                        url: node.url,
                        pushed_at: node.pushed_at,
                    }
                })
                .collect(),
        )
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
