//! Types for GitHub API responses.

/// The aggregated CI check status for a repository's default branch HEAD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Success,
    Failure,
    Pending,
}

/// Owner of a GitHub repository.
#[derive(Debug, Clone)]
pub struct RepositoryOwner {
    pub login: String,
}

/// A GitHub repository with its default-branch check status.
#[derive(Debug, Clone)]
pub struct Repository {
    pub name: String,
    pub owner: RepositoryOwner,
    /// Aggregated check status of the HEAD commit on the default branch,
    /// or `None` if no checks have run on that commit.
    pub main_check_status: Option<CheckStatus>,
}

/// An open pull request with its head-commit check status.
#[derive(Debug, Clone)]
pub struct PullRequest {
    pub number: i32,
    pub title: String,
    pub url: String,
    pub check_status: Option<CheckStatus>,
}

/// Detailed view of a repository: description and open PRs.
#[derive(Debug, Clone)]
pub struct RepoDetail {
    pub description: Option<String>,
    pub pull_requests: Vec<PullRequest>,
}
