//! GitHub OAuth flow for user authentication.
//!
//! Implements the web application flow:
//! 1. Redirect user to GitHub's authorization page
//! 2. Exchange the authorization code for an access token
//! 3. Fetch user information with the access token

use http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use http::{Method, Request, StatusCode};
use http_body_util::{BodyExt, Full};
use serde::Deserialize;
use thiserror::Error;
use url::Url;

use crate::HttpClient;
use crate::session::GitHubUser;

use cynic::QueryBuilder;

use super::graphql::{
    GitObject, IssueOrder, IssueOrderField, ListUserReposQuery, ListUserReposVariables,
    OrderDirection, PullRequestState, RepoDetailQuery, RepoDetailVariables, RepositoryAffiliation,
    RepositoryOrder, RepositoryOrderField, StatusState,
};
use super::types::{CheckStatus, PullRequest, RepoDetail, Repository, RepositoryOwner};

/// Configuration for the GitHub OAuth flow.
#[derive(Debug, Clone)]
pub struct GitHubOAuthConfig {
    /// The GitHub App's client ID.
    pub client_id: String,
    /// The GitHub App's client secret.
    pub client_secret: String,
    /// Base URL for github.com — overridable for testing.
    ///
    /// Defaults to `"https://github.com"`.
    pub github_base_url: Url,
    /// Base URL for the GitHub REST API — overridable for testing.
    ///
    /// Defaults to `"https://api.github.com"`.
    pub api_base_url: Url,
}

impl Default for GitHubOAuthConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            github_base_url: Url::parse("https://github.com").expect("hardcoded URL is valid"),
            api_base_url: Url::parse("https://api.github.com").expect("hardcoded URL is valid"),
        }
    }
}

/// Errors that can occur during the OAuth flow.
#[derive(Debug, Error)]
pub enum OAuthError<E> {
    /// Failed to build the HTTP request.
    #[error("failed to build request: {0}")]
    RequestBuild(#[from] http::Error),
    /// HTTP transport error.
    #[error("HTTP error: {0}")]
    Http(E),
    /// GitHub returned an error response.
    #[error("GitHub OAuth error ({status}): {body}")]
    ApiError {
        /// The HTTP status code.
        status: StatusCode,
        /// The response body.
        body: String,
    },
    /// Failed to parse the response.
    #[error("failed to parse response: {0}")]
    JsonParse(serde_json::Error),
    /// GitHub returned an OAuth error in the response body.
    #[error("OAuth error: {error} - {error_description}")]
    OAuthResponse {
        /// The error code.
        error: String,
        /// The error description.
        error_description: String,
    },
    /// Failed to encode form body.
    #[error("failed to encode form: {0}")]
    FormEncode(serde_urlencoded::ser::Error),
    /// The GraphQL response contained errors.
    #[error("GraphQL error: {0}")]
    GraphQL(String),
}

/// Response from the token exchange endpoint.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// Exchanges an authorization code for an access token.
pub async fn exchange_code_for_token<C>(
    http_client: &C,
    config: &GitHubOAuthConfig,
    code: &str,
) -> Result<String, OAuthError<C::Error>>
where
    C: HttpClient,
{
    let params = [
        ("client_id", config.client_id.as_str()),
        ("client_secret", config.client_secret.as_str()),
        ("code", code),
    ];
    let body = serde_urlencoded::to_string(params).map_err(OAuthError::FormEncode)?;

    let token_url = config
        .github_base_url
        .join("/login/oauth/access_token")
        .expect("valid path join");
    let request = Request::builder()
        .method(Method::POST)
        .uri(token_url.as_str())
        .header(ACCEPT, "application/json")
        .header(
            http::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(Full::new(bytes::Bytes::from(body.into_bytes())))?;

    let response = http_client.fetch(request).await.map_err(OAuthError::Http)?;
    let status = response.status();
    let body_bytes = response
        .into_body()
        .collect()
        .await
        .unwrap_or_default()
        .to_bytes();

    if !status.is_success() {
        let body_str = String::from_utf8_lossy(&body_bytes).into_owned();
        return Err(OAuthError::ApiError {
            status,
            body: body_str,
        });
    }

    let token_response: TokenResponse =
        serde_json::from_slice(&body_bytes).map_err(OAuthError::JsonParse)?;

    match token_response.access_token {
        Some(token) => Ok(token),
        None => Err(OAuthError::OAuthResponse {
            error: token_response.error.unwrap_or_default(),
            error_description: token_response.error_description.unwrap_or_default(),
        }),
    }
}

/// Fetches the authenticated user's information from GitHub.
pub async fn fetch_user<C>(
    http_client: &C,
    config: &GitHubOAuthConfig,
    access_token: &str,
) -> Result<GitHubUser, OAuthError<C::Error>>
where
    C: HttpClient,
{
    let user_url = config.api_base_url.join("/user").expect("valid path join");
    let request = Request::builder()
        .method(Method::GET)
        .uri(user_url.as_str())
        .header(ACCEPT, "application/vnd.github+json")
        .header(AUTHORIZATION, format!("Bearer {access_token}"))
        .header(USER_AGENT, "hubdash")
        .body(Full::new(bytes::Bytes::new()))?;

    let response = http_client.fetch(request).await.map_err(OAuthError::Http)?;
    let status = response.status();
    let body_bytes = response
        .into_body()
        .collect()
        .await
        .unwrap_or_default()
        .to_bytes();

    if !status.is_success() {
        let body_str = String::from_utf8_lossy(&body_bytes).into_owned();
        return Err(OAuthError::ApiError {
            status,
            body: body_str,
        });
    }

    serde_json::from_slice(&body_bytes).map_err(OAuthError::JsonParse)
}

fn status_state_to_check_status(state: StatusState) -> CheckStatus {
    match state {
        StatusState::Success => CheckStatus::Success,
        StatusState::Failure | StatusState::Error => CheckStatus::Failure,
        StatusState::Pending | StatusState::Expected => CheckStatus::Pending,
    }
}

/// Lists repositories accessible to the authenticated user via GraphQL.
///
/// Only returns non-fork repositories whose default branch HEAD commit has a
/// `statusCheckRollup` — i.e. repositories that actually have CI checks
/// configured.  The rollup state is included in each returned [`Repository`].
pub async fn list_user_repos<C>(
    http_client: &C,
    api_base_url: &Url,
    access_token: &str,
) -> Result<Vec<Repository>, OAuthError<C::Error>>
where
    C: HttpClient,
{
    let operation = ListUserReposQuery::build(ListUserReposVariables {
        first: 100,
        owner_affiliations: vec![
            RepositoryAffiliation::Owner,
            RepositoryAffiliation::OrganizationMember,
            RepositoryAffiliation::Collaborator,
        ],
        order_by: RepositoryOrder {
            field: RepositoryOrderField::PushedAt,
            direction: OrderDirection::Desc,
        },
    });

    let graphql_url = api_base_url.join("/graphql").expect("valid path join");
    let body = bytes::Bytes::from(
        serde_json::to_vec(&operation).expect("cynic operation serialization cannot fail"),
    );

    let request = Request::builder()
        .method(Method::POST)
        .uri(graphql_url.as_str())
        .header(ACCEPT, "application/vnd.github+json")
        .header(AUTHORIZATION, format!("Bearer {access_token}"))
        .header(USER_AGENT, "hubdash")
        .header(CONTENT_TYPE, "application/json")
        .body(Full::new(body))?;

    let response = http_client.fetch(request).await.map_err(OAuthError::Http)?;
    let status = response.status();
    let body_bytes = response
        .into_body()
        .collect()
        .await
        .unwrap_or_default()
        .to_bytes();

    if !status.is_success() {
        let body_str = String::from_utf8_lossy(&body_bytes).into_owned();
        return Err(OAuthError::ApiError {
            status,
            body: body_str,
        });
    }

    let gql: cynic::GraphQlResponse<ListUserReposQuery> =
        serde_json::from_slice(&body_bytes).map_err(OAuthError::JsonParse)?;

    if let Some(errors) = gql.errors {
        let msg = errors
            .into_iter()
            .map(|e| e.message)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(OAuthError::GraphQL(msg));
    }

    let data = gql
        .data
        .ok_or_else(|| OAuthError::GraphQL("response missing data field".into()))?;

    Ok(data
        .viewer
        .repositories
        .nodes
        .into_iter()
        .filter(|node| !node.is_fork)
        .filter_map(|node| {
            // Extract the statusCheckRollup from defaultBranchRef → Commit.
            // Repos with no rollup (no checks on HEAD) are excluded.
            let main_check_status = node
                .default_branch_ref
                .as_ref()
                .and_then(|r| r.target.as_ref())
                .and_then(|t| match t {
                    GitObject::Commit(c) => c.status_check_rollup.as_ref(),
                    GitObject::Other => None,
                })
                .map(|rollup| status_state_to_check_status(rollup.state));

            main_check_status.map(|status| Repository {
                name: node.name,
                owner: RepositoryOwner {
                    login: node.owner.login,
                },
                main_check_status: Some(status),
            })
        })
        .collect())
}

fn check_status_from_ref(branch_ref: Option<&super::graphql::BranchRef>) -> Option<CheckStatus> {
    branch_ref
        .and_then(|r| r.target.as_ref())
        .and_then(|t| match t {
            GitObject::Commit(c) => c.status_check_rollup.as_ref(),
            GitObject::Other => None,
        })
        .map(|rollup| status_state_to_check_status(rollup.state))
}

/// Fetches the description and open pull requests for a single repository.
///
/// Returns `Ok(None)` if GitHub reports the repository does not exist.
pub async fn fetch_repo_detail<C>(
    http_client: &C,
    api_base_url: &Url,
    access_token: &str,
    owner: &str,
    name: &str,
) -> Result<Option<RepoDetail>, OAuthError<C::Error>>
where
    C: HttpClient,
{
    let operation = RepoDetailQuery::build(RepoDetailVariables {
        owner: owner.to_string(),
        name: name.to_string(),
        pr_first: 10,
        pr_states: Some(vec![PullRequestState::Open]),
        pr_order_by: Some(IssueOrder {
            field: IssueOrderField::UpdatedAt,
            direction: OrderDirection::Desc,
        }),
    });

    let graphql_url = api_base_url.join("/graphql").expect("valid path join");
    let body = bytes::Bytes::from(
        serde_json::to_vec(&operation).expect("cynic operation serialization cannot fail"),
    );

    let request = Request::builder()
        .method(Method::POST)
        .uri(graphql_url.as_str())
        .header(ACCEPT, "application/vnd.github+json")
        .header(AUTHORIZATION, format!("Bearer {access_token}"))
        .header(USER_AGENT, "hubdash")
        .header(CONTENT_TYPE, "application/json")
        .body(Full::new(body))?;

    let response = http_client.fetch(request).await.map_err(OAuthError::Http)?;
    let status = response.status();
    let body_bytes = response
        .into_body()
        .collect()
        .await
        .unwrap_or_default()
        .to_bytes();

    if !status.is_success() {
        let body_str = String::from_utf8_lossy(&body_bytes).into_owned();
        return Err(OAuthError::ApiError {
            status,
            body: body_str,
        });
    }

    let gql: cynic::GraphQlResponse<RepoDetailQuery> =
        serde_json::from_slice(&body_bytes).map_err(OAuthError::JsonParse)?;

    if let Some(errors) = gql.errors {
        let msg = errors
            .into_iter()
            .map(|e| e.message)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(OAuthError::GraphQL(msg));
    }

    let data = gql
        .data
        .ok_or_else(|| OAuthError::GraphQL("response missing data field".into()))?;

    Ok(data.repository.map(|repo| RepoDetail {
        description: repo.description,
        pull_requests: repo
            .pull_requests
            .nodes
            .into_iter()
            .map(|pr| PullRequest {
                number: pr.number,
                title: pr.title,
                url: pr.url,
                check_status: check_status_from_ref(pr.head_ref.as_ref()),
            })
            .collect(),
    }))
}
