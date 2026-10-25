use crate::database::models::Repository;
use crate::errors::Error;
use chrono::{DateTime, Utc};
use reqwest::blocking::{Client, Response};
use reqwest::header::USER_AGENT;
use std::str::FromStr;

const GITHUB_HOST: &str = "api.github.com";
const GITHUB_GRAPHQL_URL: &str = "https://api.github.com/graphql";
const REPOSITORIES_QUERY: &str = "query($after: String) { \
    viewer { \
        repositories(first: 100, after: $after, affiliations: [OWNER, COLLABORATOR, ORGANIZATION_MEMBER], ownerAffiliations: [OWNER, COLLABORATOR, ORGANIZATION_MEMBER]) { \
            pageInfo { hasNextPage endCursor } \
            edges { node { pushedAt url } } \
        } \
    } \
}";

/// A GitHub token and the GraphQL endpoint it authenticates against.
///
/// Parsed from `<token>` (github.com) or `<token>@<host>` (GitHub Enterprise Server).
pub struct Account {
    token: String,
    host: String,
    graphql_url: String,
}

impl FromStr for Account {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (token, host, graphql_url) = match s.split_once('@') {
            None => (s, GITHUB_HOST, GITHUB_GRAPHQL_URL.to_string()),
            Some((_, "")) => return Err(Error::InvalidAccount("empty host after '@'")),
            Some((token, host)) => (token, host, format!("https://{host}/api/graphql")),
        };
        if token.is_empty() {
            return Err(Error::InvalidAccount("empty token"));
        }
        Ok(Self {
            token: token.to_string(),
            host: host.to_string(),
            graphql_url,
        })
    }
}

/// Parses a comma-separated list of accounts, e.g. `tok1,tok2@ghe.corp.com`.
///
/// # Errors
///
/// Will return `Err` if the list is empty or any entry is malformed.
pub fn parse_accounts(s: &str) -> Result<Vec<Account>, Error> {
    let accounts = s
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(Account::from_str)
        .collect::<Result<Vec<_>, _>>()?;
    if accounts.is_empty() {
        return Err(Error::InvalidAccount("no API keys provided"));
    }
    Ok(accounts)
}

pub struct GitHubAPI<'a> {
    account: &'a Account,
    client: Client,
}

impl<'a> GitHubAPI<'a> {
    #[inline]
    pub fn new(account: &'a Account) -> Self {
        Self {
            account,
            client: Client::new(),
        }
    }

    #[inline]
    pub const fn accessible_repositories(&self) -> OwnedRepositories<'_> {
        OwnedRepositories {
            api: self,
            has_more: true,
            cursor: None,
        }
    }

    #[inline]
    fn fetch_repositories(&self, after: Option<&str>) -> Result<Results, Error> {
        let body = GraphQlRequest {
            query: REPOSITORIES_QUERY,
            variables: Variables { after },
        };
        self.client
            .post(&self.account.graphql_url)
            .bearer_auth(&self.account.token)
            .header(USER_AGENT, "Alfred Github Workflow")
            .json(&body)
            .send()
            .and_then(Response::error_for_status)
            .and_then(Response::json)
            .map_err(|source| Error::Request {
                host: self.account.host.clone(),
                source,
            })
    }
}

pub struct OwnedRepositories<'a> {
    api: &'a GitHubAPI<'a>,
    has_more: bool,
    cursor: Option<String>,
}

impl Iterator for OwnedRepositories<'_> {
    type Item = Result<Vec<Repository>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.has_more {
            return None;
        }
        let results = match self.api.fetch_repositories(self.cursor.as_deref()) {
            Ok(results) => results,
            Err(e) => {
                self.has_more = false;
                return Some(Err(e));
            }
        };
        let repositories = results.data.viewer.repositories;
        self.has_more = repositories.page_info.has_next_page;
        self.cursor = repositories.page_info.end_cursor;
        Some(Ok(repositories
            .edges
            .into_iter()
            .filter_map(|edge| edge?.node)
            .map(|node| {
                let mut s = node.url.rsplit('/');
                let name = s.next().unwrap_or_default().to_string();
                let owner = s.next().unwrap_or_default();
                Repository {
                    name_with_owner: format!("{owner}/{name}"),
                    name,
                    url: node.url,
                    pushed_at: node.pushed_at,
                }
            })
            .collect()))
    }
}

#[derive(Serialize)]
struct GraphQlRequest<'a> {
    query: &'static str,
    variables: Variables<'a>,
}

#[derive(Serialize)]
struct Variables<'a> {
    after: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct PageInfo {
    #[serde(rename = "endCursor")]
    end_cursor: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn urls(s: &str) -> Vec<String> {
        parse_accounts(s)
            .unwrap()
            .into_iter()
            .map(|a| a.graphql_url)
            .collect()
    }

    #[test]
    fn single_token_targets_github_com() {
        let accounts = parse_accounts("ghp_abc").unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].token, "ghp_abc");
        assert_eq!(accounts[0].graphql_url, GITHUB_GRAPHQL_URL);
    }

    #[test]
    fn comma_list_with_whitespace_and_trailing_comma() {
        assert_eq!(
            urls(" ghp_a , github_pat_b,ghp_c@ghe.corp.com ,"),
            [
                GITHUB_GRAPHQL_URL,
                GITHUB_GRAPHQL_URL,
                "https://ghe.corp.com/api/graphql"
            ]
        );
    }

    #[test]
    fn host_suffix_targets_ghes() {
        let accounts = parse_accounts("ghp_c@ghe.corp.com").unwrap();
        assert_eq!(accounts[0].token, "ghp_c");
        assert_eq!(accounts[0].host, "ghe.corp.com");
    }

    #[test]
    fn rejects_empty_list() {
        assert!(matches!(
            parse_accounts(" , "),
            Err(Error::InvalidAccount(_))
        ));
    }

    #[test]
    fn rejects_empty_host() {
        assert!(matches!(
            parse_accounts("ghp_a@"),
            Err(Error::InvalidAccount(_))
        ));
    }

    #[test]
    fn rejects_empty_token() {
        assert!(matches!(
            parse_accounts("@ghe.corp.com"),
            Err(Error::InvalidAccount(_))
        ));
    }
}
