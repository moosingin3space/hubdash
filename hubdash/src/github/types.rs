//! Types for GitHub API responses.

use serde::Deserialize;

/// The aggregated CI check status for a repository's default branch HEAD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    /// All checks passed.
    Success,
    /// At least one check failed or errored.
    Failure,
    /// Checks are queued or in progress.
    Pending,
}

/// Owner of a GitHub repository.
#[derive(Debug, Clone, Deserialize)]
pub struct RepositoryOwner {
    /// The login name of the owner.
    pub login: String,
}

/// A GitHub repository.
#[derive(Debug, Clone)]
pub struct Repository {
    /// The short name of the repository (e.g. "hubdash").
    pub name: String,
    /// The owner of the repository.
    pub owner: RepositoryOwner,
    /// The repository description.
    pub description: Option<String>,
    /// Aggregated check status of the HEAD commit on the default branch,
    /// or `None` if no checks have run on that commit.
    pub main_check_status: Option<CheckStatus>,
}
