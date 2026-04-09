//! Minimal mock GitHub OAuth servers running inside turmoil.
//!
//! # Design
//!
//! Two separate Axum routers mimic the two GitHub hostnames used by the OAuth
//! flow:
//!
//! - **`github.com`** (via [`mock_github_router`]):
//!   `POST /login/oauth/access_token` — returns [`MOCK_ACCESS_TOKEN`]
//!
//! - **`api.github.com`** (via [`mock_api_github_router`]):
//!   `GET /user` — returns a canned [`GitHubUser`] payload
//!   `POST /graphql` — served by an [`async_graphql`] schema that mirrors the
//!   subset of the GitHub GraphQL API used by hubdash
//!
//! Both are plain HTTP so turmoil can intercept the connections without TLS.
//!
//! # Usage
//!
//! ```ignore
//! register_github_server(&mut sim);     // github.com:80
//! register_api_github_server(&mut sim); // api.github.com:80
//! ```

use async_graphql::{
    Context, EmptyMutation, EmptySubscription, Enum, InputObject, Interface, Object, Schema,
    SimpleObject,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::{
    Router,
    routing::{get, post},
};
use http::StatusCode;

/// The canned access token the mock token endpoint issues.
pub const MOCK_ACCESS_TOKEN: &str = "mock-access-token-123";

/// The canned GitHub user login.
pub const MOCK_USER_LOGIN: &str = "test-user";

/// The canned GitHub user ID.
pub const MOCK_USER_ID: u64 = 42;

/// Hostname for the mock `api.github.com` used inside turmoil.
pub const API_GITHUB_HOST: &str = "api.github.com";

// ── github.com router ────────────────────────────────────────────────────────

/// Router for the mock `github.com` host.
///
/// Handles `POST /login/oauth/access_token`.
pub fn mock_github_router() -> Router {
    Router::new().route("/login/oauth/access_token", post(mock_token_endpoint))
}

/// Returns a successful token exchange response regardless of the posted code.
async fn mock_token_endpoint() -> impl IntoResponse {
    let body = serde_json::json!({
        "access_token": MOCK_ACCESS_TOKEN,
        "token_type": "bearer",
        "scope": "read:user"
    });
    (StatusCode::OK, axum::Json(body))
}

// ── async-graphql schema ─────────────────────────────────────────────────────
//
// This mirrors the subset of the GitHub GraphQL API described in
// `schemas/github.graphql` so that the cynic query in `oauth.rs` can be
// exercised end-to-end in turmoil tests.

type MockSchema = Schema<MockQuery, EmptyMutation, EmptySubscription>;

/// Repository affiliation — matches the schema enum.
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
enum RepositoryAffiliation {
    Owner,
    Collaborator,
    OrganizationMember,
}

/// Repository sort field — matches the schema enum.
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
enum RepositoryOrderField {
    PushedAt,
    UpdatedAt,
    CreatedAt,
    Name,
    Stargazers,
}

/// Sort direction — matches the schema enum.
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
enum OrderDirection {
    Asc,
    Desc,
}

/// Ordering argument for `repositories`.
#[derive(InputObject, Clone)]
struct RepositoryOrder {
    field: RepositoryOrderField,
    direction: OrderDirection,
}

/// Possible states for a commit's combined check rollup — mirrors `StatusState`.
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(name = "StatusState")]
enum MockStatusState {
    Error,
    Expected,
    Failure,
    Pending,
    Success,
}

/// Combined check status rollup — mirrors `StatusCheckRollup`.
#[derive(SimpleObject, Clone)]
#[graphql(name = "StatusCheckRollup")]
struct MockStatusCheckRollup {
    state: MockStatusState,
}

/// Mock `Commit` object — implements the `GitObject` interface.
#[derive(Clone)]
struct MockCommit {
    has_checks: bool,
}

#[Object(name = "Commit")]
impl MockCommit {
    async fn id(&self) -> String {
        "mock-commit-id".to_string()
    }

    async fn status_check_rollup(&self) -> Option<MockStatusCheckRollup> {
        self.has_checks.then_some(MockStatusCheckRollup {
            state: MockStatusState::Success,
        })
    }
}

/// `GitObject` interface — the only implementor we expose is `Commit`.
#[derive(Interface, Clone)]
#[graphql(field(name = "id", ty = "String"))]
enum MockGitObject {
    MockCommit(MockCommit),
}

/// `Ref` type — wraps the target `GitObject` of a branch ref.
#[derive(SimpleObject, Clone)]
#[graphql(name = "Ref")]
struct MockRef {
    target: Option<MockGitObject>,
}

/// Repository owner (user or organisation).
#[derive(SimpleObject, Clone)]
struct MockOwner {
    login: String,
}

/// A repository node returned by the mock viewer query.
#[derive(SimpleObject, Clone)]
struct MockRepo {
    database_id: Option<i32>,
    name: String,
    owner: MockOwner,
    description: Option<String>,
    is_private: bool,
    url: String,
    is_fork: bool,
    default_branch_ref: Option<MockRef>,
}

/// `nodes` list for the repository connection.
#[derive(SimpleObject)]
struct MockRepositoryConnection {
    nodes: Vec<MockRepo>,
}

/// The authenticated viewer.
struct MockViewer;

#[Object]
impl MockViewer {
    /// Returns hardcoded mock repositories, ignoring all filter arguments.
    async fn repositories(
        &self,
        _ctx: &Context<'_>,
        _first: i32,
        _owner_affiliations: Vec<RepositoryAffiliation>,
        _order_by: RepositoryOrder,
    ) -> MockRepositoryConnection {
        MockRepositoryConnection {
            nodes: mock_repos(),
        }
    }
}

/// Root query type.
struct MockQuery;

#[Object]
impl MockQuery {
    async fn viewer(&self, _ctx: &Context<'_>) -> MockViewer {
        MockViewer
    }
}

/// Hardcoded repositories returned by the mock GraphQL endpoint.
///
/// Includes one fork (`forked-lib`) to verify that the application filters
/// forks out.  Repos with `has_checks: false` are excluded because their
/// `statusCheckRollup` returns `None`.
fn mock_repos() -> Vec<MockRepo> {
    fn repo(id: i32, name: &str, is_fork: bool, has_checks: bool) -> MockRepo {
        MockRepo {
            database_id: Some(id),
            name: name.to_string(),
            owner: MockOwner {
                login: MOCK_USER_LOGIN.to_string(),
            },
            description: None,
            is_private: false,
            url: format!("https://github.com/{MOCK_USER_LOGIN}/{name}"),
            is_fork,
            default_branch_ref: Some(MockRef {
                target: Some(MockGitObject::MockCommit(MockCommit { has_checks })),
            }),
        }
    }

    vec![
        repo(1, "hubdash", false, true),
        repo(2, "api-gateway", false, true),
        repo(3, "frontend-app", false, true),
        repo(4, "forked-lib", true, false),
    ]
}

// ── api.github.com router ────────────────────────────────────────────────────

/// Router for the mock `api.github.com` host.
///
/// Handles `GET /user` and `POST /graphql`.
pub fn mock_api_github_router() -> Router {
    let schema = Schema::build(MockQuery, EmptyMutation, EmptySubscription).finish();
    Router::new()
        .route("/user", get(mock_user_endpoint))
        .route("/graphql", post(mock_graphql_endpoint))
        .with_state(schema)
}

/// Returns a canned GitHub user object.
async fn mock_user_endpoint() -> impl IntoResponse {
    let body = serde_json::json!({
        "id": MOCK_USER_ID,
        "login": MOCK_USER_LOGIN,
        "name": "Test User",
        "avatar_url": null
    });
    (StatusCode::OK, axum::Json(body))
}

/// Executes the incoming GraphQL request against the mock schema and returns
/// the response.
async fn mock_graphql_endpoint(
    State(schema): State<MockSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}
