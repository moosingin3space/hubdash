//! Minimal mock GitHub OAuth servers running inside turmoil.

use async_graphql::{
    ComplexObject, Context, EmptyMutation, EmptySubscription, Enum, InputObject, Interface, Object,
    Schema, SimpleObject,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::{
    Router,
    routing::{get, post},
};
use http::StatusCode;

pub const MOCK_ACCESS_TOKEN: &str = "mock-access-token-123";
pub const MOCK_USER_LOGIN: &str = "test-user";
pub const MOCK_USER_ID: u64 = 42;
pub const API_GITHUB_HOST: &str = "api.github.com";

// ── github.com router ────────────────────────────────────────────────────────

pub fn mock_github_router() -> Router {
    Router::new().route("/login/oauth/access_token", post(mock_token_endpoint))
}

async fn mock_token_endpoint() -> impl IntoResponse {
    let body = serde_json::json!({
        "access_token": MOCK_ACCESS_TOKEN,
        "token_type": "bearer",
        "scope": "read:user"
    });
    (StatusCode::OK, axum::Json(body))
}

// ── Shared enum types ─────────────────────────────────────────────────────────

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
enum RepositoryAffiliation {
    Owner,
    Collaborator,
    OrganizationMember,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
enum RepositoryOrderField {
    PushedAt,
    UpdatedAt,
    CreatedAt,
    Name,
    Stargazers,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
enum OrderDirection {
    Asc,
    Desc,
}

#[derive(InputObject, Clone)]
struct RepositoryOrder {
    field: RepositoryOrderField,
    direction: OrderDirection,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(name = "StatusState")]
enum MockStatusState {
    Error,
    Expected,
    Failure,
    Pending,
    Success,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(name = "PullRequestState")]
enum MockPullRequestState {
    Open,
    Closed,
    Merged,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(name = "IssueOrderField")]
enum MockIssueOrderField {
    CreatedAt,
    UpdatedAt,
    Comments,
}

#[derive(InputObject, Clone)]
#[graphql(name = "IssueOrder")]
struct MockIssueOrder {
    field: MockIssueOrderField,
    direction: OrderDirection,
}

// ── Shared object types ───────────────────────────────────────────────────────

#[derive(SimpleObject, Clone)]
#[graphql(name = "StatusCheckRollup")]
struct MockStatusCheckRollup {
    state: MockStatusState,
}

/// Mock `Commit` — implements `GitObject` interface.
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

/// `GitObject` interface.
#[derive(Interface, Clone)]
#[graphql(field(name = "id", ty = "String"))]
enum MockGitObject {
    MockCommit(MockCommit),
}

/// `Ref` type.
#[derive(SimpleObject, Clone)]
#[graphql(name = "Ref")]
struct MockRef {
    target: Option<MockGitObject>,
}

fn commit_ref(has_checks: bool) -> Option<MockRef> {
    Some(MockRef {
        target: Some(MockGitObject::MockCommit(MockCommit { has_checks })),
    })
}

// ── Pull request types ────────────────────────────────────────────────────────

struct MockPullRequest {
    number: i32,
    title: String,
    url: String,
    has_checks: bool,
}

#[Object(name = "PullRequest")]
impl MockPullRequest {
    async fn number(&self) -> i32 {
        self.number
    }
    async fn title(&self) -> &str {
        &self.title
    }
    async fn url(&self) -> &str {
        &self.url
    }
    async fn head_ref(&self) -> Option<MockRef> {
        commit_ref(self.has_checks)
    }
}

#[derive(SimpleObject)]
#[graphql(name = "PullRequestConnection")]
struct MockPullRequestConnection {
    nodes: Vec<MockPullRequest>,
}

// ── Repository type ───────────────────────────────────────────────────────────

#[derive(SimpleObject, Clone)]
#[graphql(complex)]
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

#[ComplexObject]
impl MockRepo {
    /// Accepts (and ignores) the pagination/filter arguments from the cynic query.
    async fn pull_requests(
        &self,
        _first: i32,
        _states: Option<Vec<MockPullRequestState>>,
        _order_by: Option<MockIssueOrder>,
    ) -> MockPullRequestConnection {
        MockPullRequestConnection {
            nodes: mock_prs_for(&self.name),
        }
    }
}

#[derive(SimpleObject, Clone)]
struct MockOwner {
    login: String,
}

#[derive(SimpleObject)]
struct MockRepositoryConnection {
    nodes: Vec<MockRepo>,
}

// ── Hardcoded data ────────────────────────────────────────────────────────────

fn mock_repos() -> Vec<MockRepo> {
    fn repo(id: i32, name: &str, is_fork: bool, has_checks: bool) -> MockRepo {
        MockRepo {
            database_id: Some(id),
            name: name.to_string(),
            owner: MockOwner {
                login: MOCK_USER_LOGIN.to_string(),
            },
            description: Some(format!("Mock description for {name}")),
            is_private: false,
            url: format!("https://github.com/{MOCK_USER_LOGIN}/{name}"),
            is_fork,
            default_branch_ref: commit_ref(has_checks),
        }
    }

    vec![
        repo(1, "hubdash", false, true),
        repo(2, "api-gateway", false, true),
        repo(3, "frontend-app", false, true),
        repo(4, "forked-lib", true, false),
    ]
}

fn mock_prs_for(repo_name: &str) -> Vec<MockPullRequest> {
    if repo_name == "hubdash" {
        vec![
            MockPullRequest {
                number: 42,
                title: "Add feature X".to_string(),
                url: format!("https://github.com/{MOCK_USER_LOGIN}/hubdash/pull/42"),
                has_checks: true,
            },
            MockPullRequest {
                number: 41,
                title: "Fix bug Y".to_string(),
                url: format!("https://github.com/{MOCK_USER_LOGIN}/hubdash/pull/41"),
                has_checks: true,
            },
        ]
    } else {
        vec![]
    }
}

// ── Query resolvers ───────────────────────────────────────────────────────────

struct MockViewer;

#[Object]
impl MockViewer {
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

struct MockQuery;

#[Object]
impl MockQuery {
    async fn viewer(&self, _ctx: &Context<'_>) -> MockViewer {
        MockViewer
    }

    async fn repository(&self, owner: String, name: String) -> Option<MockRepo> {
        mock_repos()
            .into_iter()
            .find(|r| r.owner.login == owner && r.name == name)
    }
}

// ── api.github.com router ────────────────────────────────────────────────────

type MockSchema = Schema<MockQuery, EmptyMutation, EmptySubscription>;

pub fn mock_api_github_router() -> Router {
    let schema = Schema::build(MockQuery, EmptyMutation, EmptySubscription).finish();
    Router::new()
        .route("/user", get(mock_user_endpoint))
        .route("/graphql", post(mock_graphql_endpoint))
        .with_state(schema)
}

async fn mock_user_endpoint() -> impl IntoResponse {
    let body = serde_json::json!({
        "id": MOCK_USER_ID,
        "login": MOCK_USER_LOGIN,
        "name": "Test User",
        "avatar_url": null
    });
    (StatusCode::OK, axum::Json(body))
}

async fn mock_graphql_endpoint(
    State(schema): State<MockSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}
