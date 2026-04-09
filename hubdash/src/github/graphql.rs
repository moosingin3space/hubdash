//! Cynic query definitions for the GitHub GraphQL API.
//!
//! The types here are validated against `schemas/github.graphql` at compile
//! time via the build script.  Adding or removing fields, misspelling a type
//! name, or referencing a non-existent argument will all be caught before the
//! binary is built.

// Binds this module to the registered "github" schema so that all
// `#[derive(cynic::*)]` items below can reference it without repeating
// `schema_path` on every attribute.
#[cynic::schema("github")]
mod schema {}

// ── Shared enums / input types ────────────────────────────────────────────────

/// Repository affiliation filter for the `viewer.repositories` query.
#[derive(cynic::Enum, Clone, Debug)]
pub(crate) enum RepositoryAffiliation {
    Owner,
    Collaborator,
    OrganizationMember,
}

#[derive(cynic::Enum, Clone, Debug)]
pub(crate) enum RepositoryOrderField {
    PushedAt,
    UpdatedAt,
    CreatedAt,
    Name,
    Stargazers,
}

#[derive(cynic::Enum, Clone, Debug)]
pub(crate) enum OrderDirection {
    Asc,
    Desc,
}

#[derive(cynic::InputObject, Clone, Debug)]
pub(crate) struct RepositoryOrder {
    pub(crate) field: RepositoryOrderField,
    pub(crate) direction: OrderDirection,
}

// ── Shared fragments (reused by both queries) ─────────────────────────────────

/// The combined check-and-status-context rollup for a commit.
#[derive(cynic::QueryFragment, Debug)]
pub(crate) struct StatusCheckRollup {
    pub(crate) state: StatusState,
}

/// Possible states for a commit's combined check rollup.
#[derive(cynic::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StatusState {
    Error,
    Expected,
    Failure,
    Pending,
    Success,
}

/// The `Commit` concrete type, selected via an inline fragment on `GitObject`.
#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Commit")]
pub(crate) struct CommitNode {
    pub(crate) status_check_rollup: Option<StatusCheckRollup>,
}

/// Inline-fragment selector over the `GitObject` interface.
#[derive(cynic::InlineFragments, Debug)]
pub(crate) enum GitObject {
    Commit(CommitNode),
    #[cynic(fallback)]
    Other,
}

/// Wraps the target `GitObject` of any branch / head ref.
#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Ref")]
pub(crate) struct BranchRef {
    pub(crate) target: Option<GitObject>,
}

// ── viewer.repositories query ─────────────────────────────────────────────────

#[derive(cynic::QueryVariables, Debug)]
pub(crate) struct ListUserReposVariables {
    pub(crate) first: i32,
    pub(crate) owner_affiliations: Vec<RepositoryAffiliation>,
    pub(crate) order_by: RepositoryOrder,
}

#[derive(cynic::QueryFragment, Debug)]
pub(crate) struct RepositoryOwner {
    pub(crate) login: String,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Repository")]
pub(crate) struct RepoNode {
    pub(crate) name: String,
    pub(crate) owner: RepositoryOwner,
    pub(crate) is_fork: bool,
    pub(crate) default_branch_ref: Option<BranchRef>,
}

#[derive(cynic::QueryFragment, Debug)]
pub(crate) struct RepositoryConnection {
    pub(crate) nodes: Vec<RepoNode>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "User", variables = "ListUserReposVariables")]
pub(crate) struct Viewer {
    #[arguments(first: $first, ownerAffiliations: $owner_affiliations, orderBy: $order_by)]
    pub(crate) repositories: RepositoryConnection,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "ListUserReposVariables")]
pub(crate) struct ListUserReposQuery {
    pub(crate) viewer: Viewer,
}

// ── repository(owner, name) detail query ─────────────────────────────────────

#[derive(cynic::QueryVariables, Debug)]
pub(crate) struct RepoDetailVariables {
    pub(crate) owner: String,
    pub(crate) name: String,
    pub(crate) pr_first: i32,
    pub(crate) pr_states: Option<Vec<PullRequestState>>,
    pub(crate) pr_order_by: Option<IssueOrder>,
}

#[derive(cynic::Enum, Clone, Debug)]
pub(crate) enum PullRequestState {
    Open,
    Closed,
    Merged,
}

#[derive(cynic::Enum, Clone, Debug)]
pub(crate) enum IssueOrderField {
    CreatedAt,
    UpdatedAt,
    Comments,
}

#[derive(cynic::InputObject, Clone, Debug)]
pub(crate) struct IssueOrder {
    pub(crate) field: IssueOrderField,
    pub(crate) direction: OrderDirection,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "PullRequest")]
pub(crate) struct PullRequestNode {
    pub(crate) number: i32,
    pub(crate) title: String,
    pub(crate) url: String,
    /// Head ref — used to reach the PR's HEAD commit check status.
    pub(crate) head_ref: Option<BranchRef>,
}

#[derive(cynic::QueryFragment, Debug)]
pub(crate) struct PullRequestConnection {
    pub(crate) nodes: Vec<PullRequestNode>,
}

/// Repository fragment used in the detail query.
#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Repository", variables = "RepoDetailVariables")]
pub(crate) struct RepoDetailFragment {
    pub(crate) description: Option<String>,
    #[arguments(first: $pr_first, states: $pr_states, orderBy: $pr_order_by)]
    pub(crate) pull_requests: PullRequestConnection,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "RepoDetailVariables")]
pub(crate) struct RepoDetailQuery {
    #[arguments(owner: $owner, name: $name)]
    pub(crate) repository: Option<RepoDetailFragment>,
}
