//! GitHub API client for making authenticated requests.

use crate::HttpClient;
use http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use http::{Method, Request, StatusCode};
use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

use super::auth::{AppCredentials, AppCredentialsError};
use super::types::{Installation, InstallationAccessToken};

/// Unit type to represent no request body.
#[derive(Serialize)]
struct NoBody;

/// The GitHub API version header.
const GITHUB_API_VERSION: &str = "2022-11-28";
/// The GitHub API base URL.
const GITHUB_API_BASE: &str = "https://api.github.com";

/// A client for interacting with the GitHub API as a GitHub App.
pub struct GitHubClient<C> {
    credentials: AppCredentials,
    http_client: C,
}

/// Errors that can occur when making GitHub API requests.
#[derive(Debug, Error)]
pub enum GitHubClientError<E> {
    #[error("authentication error: {0}")]
    Authentication(#[from] AppCredentialsError),
    #[error("HTTP error: {0}")]
    Http(E),
    #[error("failed to build request: {0}")]
    RequestBuild(#[from] http::Error),
    #[error("GitHub API error ({status}): {body}")]
    ApiError { status: StatusCode, body: String },
    #[error("failed to serialize request: {0}")]
    JsonSerialize(#[from] serde_json::Error),
    #[error("failed to parse response: {0}")]
    JsonParse(serde_json::Error),
}

impl<C> GitHubClient<C>
where
    C: HttpClient,
{
    /// Creates a new GitHub client with the given credentials and HTTP client.
    pub fn new(credentials: AppCredentials, http_client: C) -> Self {
        Self {
            credentials,
            http_client,
        }
    }

    /// Lists all installations of the GitHub App.
    ///
    /// This authenticates as the app itself using a JWT.
    pub async fn list_installations(
        &self,
    ) -> Result<Vec<Installation>, GitHubClientError<C::Error>> {
        self.app_request::<_, NoBody>(Method::GET, "/app/installations", None)
            .await
    }

    /// Gets the installation for a specific organization.
    ///
    /// This authenticates as the app itself using a JWT.
    pub async fn get_org_installation(
        &self,
        org: &str,
    ) -> Result<Installation, GitHubClientError<C::Error>> {
        self.app_request::<_, NoBody>(Method::GET, &format!("/orgs/{org}/installation"), None)
            .await
    }

    /// Gets the installation for a specific user.
    ///
    /// This authenticates as the app itself using a JWT.
    pub async fn get_user_installation(
        &self,
        username: &str,
    ) -> Result<Installation, GitHubClientError<C::Error>> {
        self.app_request::<_, NoBody>(
            Method::GET,
            &format!("/users/{username}/installation"),
            None,
        )
        .await
    }

    /// Creates an installation access token for the given installation ID.
    ///
    /// The returned token can be used to make API requests on behalf of
    /// the installation. The token expires after 1 hour.
    pub async fn create_installation_access_token(
        &self,
        installation_id: u64,
    ) -> Result<InstallationAccessToken, GitHubClientError<C::Error>> {
        self.app_request(
            Method::POST,
            &format!("/app/installations/{installation_id}/access_tokens"),
            Some(&serde_json::json!({})),
        )
        .await
    }

    /// Makes an authenticated request to the GitHub API as the app.
    async fn app_request<T, B>(
        &self,
        method: Method,
        path: &str,
        json_body: Option<&B>,
    ) -> Result<T, GitHubClientError<C::Error>>
    where
        T: DeserializeOwned,
        B: Serialize,
    {
        let jwt = self
            .credentials
            .generate_jwt()
            .map_err(GitHubClientError::Authentication)?;

        let request = self.build_request(method, path, &jwt, json_body)?;
        self.execute_request(request).await
    }

    /// Builds an HTTP request with standard GitHub API headers.
    fn build_request<B>(
        &self,
        method: Method,
        path: &str,
        token: &str,
        json_body: Option<&B>,
    ) -> Result<Request<Vec<u8>>, GitHubClientError<C::Error>>
    where
        B: Serialize,
    {
        let url = format!("{GITHUB_API_BASE}{path}");
        let (body, has_body): (Vec<u8>, bool) = match json_body {
            Some(b) => {
                let bytes = serde_json::to_vec(b).map_err(GitHubClientError::JsonSerialize)?;
                (bytes, true)
            }
            None => (Vec::new(), false),
        };

        let mut builder = Request::builder()
            .method(method)
            .uri(url)
            .header(ACCEPT, "application/vnd.github+json")
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header(USER_AGENT, "hubdash")
            .header("X-GitHub-Api-Version", GITHUB_API_VERSION);

        if has_body {
            builder = builder.header(CONTENT_TYPE, "application/json");
        }

        builder.body(body).map_err(GitHubClientError::RequestBuild)
    }

    /// Executes an HTTP request and parses the JSON response.
    async fn execute_request<T>(
        &self,
        request: Request<Vec<u8>>,
    ) -> Result<T, GitHubClientError<C::Error>>
    where
        T: DeserializeOwned,
    {
        let response = self
            .http_client
            .fetch(request)
            .await
            .map_err(GitHubClientError::Http)?;

        let status = response.status();
        let body_bytes = response.body().as_ref();

        if !status.is_success() {
            let body_str = String::from_utf8_lossy(body_bytes).into_owned();
            return Err(GitHubClientError::ApiError {
                status,
                body: body_str,
            });
        }

        serde_json::from_slice(body_bytes).map_err(GitHubClientError::JsonParse)
    }
}
