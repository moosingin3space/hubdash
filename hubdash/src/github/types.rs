//! Types for GitHub API responses.

use serde::Deserialize;

/// Response wrapper for the workflow runs list endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowRunsResponse {
    /// The list of workflow runs.
    pub workflow_runs: Vec<WorkflowRun>,
}

/// A single workflow run from the GitHub Actions API.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowRun {
    /// The name of the workflow.
    pub name: Option<String>,
    /// The event that triggered the run (e.g. "push", "pull_request", "schedule").
    pub event: String,
    /// The run status (e.g. "completed", "in_progress", "queued").
    pub status: String,
    /// The conclusion of a completed run (e.g. "success", "failure", "cancelled").
    pub conclusion: Option<String>,
    /// URL to the run on GitHub.
    pub html_url: String,
    /// When the run started, in ISO 8601 format.
    pub run_started_at: Option<String>,
    /// When the run was last updated, in ISO 8601 format.
    pub updated_at: String,
}

/// Owner of a GitHub repository.
#[derive(Debug, Clone, Deserialize)]
pub struct RepositoryOwner {
    /// The login name of the owner.
    pub login: String,
}

/// A GitHub repository.
#[derive(Debug, Clone, Deserialize)]
pub struct Repository {
    /// The short name of the repository (e.g. "hubdash").
    pub name: String,
    /// The owner of the repository.
    pub owner: RepositoryOwner,
    /// The repository description.
    pub description: Option<String>,
}
