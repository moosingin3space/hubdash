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

/// Repository affiliation filter for the `viewer.repositories` query.
#[derive(cynic::Enum, Clone, Debug)]
pub(crate) enum RepositoryAffiliation {
    /// Repositories owned directly by the viewer.
    Owner,
    /// Repositories the viewer collaborates on directly.
    Collaborator,
    /// Repositories in organisations the viewer is a member of.
    OrganizationMember,
}

/// Which field to sort repositories by.
#[derive(cynic::Enum, Clone, Debug)]
pub(crate) enum RepositoryOrderField {
    /// Sort by the most recent push date.
    PushedAt,
    /// Sort by the most recent update date.
    UpdatedAt,
    /// Sort by creation date.
    CreatedAt,
    /// Sort alphabetically by name.
    Name,
    /// Sort by star count.
    Stargazers,
}

/// Sort direction for repository listings.
#[derive(cynic::Enum, Clone, Debug)]
pub(crate) enum OrderDirection {
    /// Ascending order.
    Asc,
    /// Descending order.
    Desc,
}

/// Ordering argument for `viewer.repositories`.
#[derive(cynic::InputObject, Clone, Debug)]
pub(crate) struct RepositoryOrder {
    /// The field to order by.
    pub(crate) field: RepositoryOrderField,
    /// The direction to order in.
    pub(crate) direction: OrderDirection,
}

/// Variables for [`ListUserReposQuery`].
#[derive(cynic::QueryVariables, Debug)]
pub(crate) struct ListUserReposVariables {
    /// Maximum number of repositories to return (up to 100).
    pub(crate) first: i32,
    /// Affiliation filters — controls which repos are included.
    pub(crate) owner_affiliations: Vec<RepositoryAffiliation>,
    /// How to order the returned repositories.
    pub(crate) order_by: RepositoryOrder,
}

/// The `login` field on a repository owner.
#[derive(cynic::QueryFragment, Debug)]
pub(crate) struct RepositoryOwner {
    /// The owner's GitHub login.
    pub(crate) login: String,
}

/// The combined check-and-status-context rollup for a commit.
#[derive(cynic::QueryFragment, Debug)]
pub(crate) struct StatusCheckRollup {
    /// Aggregate state across all check suites and status contexts.
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
    /// Combined check status for this commit's default branch HEAD.
    pub(crate) status_check_rollup: Option<StatusCheckRollup>,
}

/// Inline-fragment selector over the `GitObject` interface.
///
/// We only care about `Commit` nodes; all other implementors (Tree, Blob, Tag)
/// fall through to `Other`.
#[derive(cynic::InlineFragments, Debug)]
pub(crate) enum GitObject {
    Commit(CommitNode),
    #[cynic(fallback)]
    Other,
}

/// The `Ref` type — wraps the target `GitObject` of a branch ref.
#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Ref")]
pub(crate) struct BranchRef {
    /// The Git object this ref points to (typically a `Commit`).
    pub(crate) target: Option<GitObject>,
}

/// A single repository node returned by the viewer query.
#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Repository")]
pub(crate) struct RepoNode {
    /// Short repository name.
    pub(crate) name: String,
    /// Owner login.
    pub(crate) owner: RepositoryOwner,
    /// Optional description.
    pub(crate) description: Option<String>,
    /// Whether this repository is a fork.
    pub(crate) is_fork: bool,
    /// The default branch ref — used to reach the HEAD commit's check rollup.
    pub(crate) default_branch_ref: Option<BranchRef>,
}

/// The `nodes` connection for a repository list.
#[derive(cynic::QueryFragment, Debug)]
pub(crate) struct RepositoryConnection {
    /// Repositories in this page.
    pub(crate) nodes: Vec<RepoNode>,
}

/// The authenticated `viewer` with their repository list.
#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "User", variables = "ListUserReposVariables")]
pub(crate) struct Viewer {
    /// Repositories accessible to the viewer.
    #[arguments(first: $first, ownerAffiliations: $owner_affiliations, orderBy: $order_by)]
    pub(crate) repositories: RepositoryConnection,
}

/// Root query — fetches the viewer's non-fork repositories that have CI checks.
#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "ListUserReposVariables")]
pub(crate) struct ListUserReposQuery {
    /// The authenticated viewer.
    pub(crate) viewer: Viewer,
}
