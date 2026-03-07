//! GitHub API client for GitHub App authentication and API access.
//!
//! This module provides the core functionality for authenticating as a GitHub App
//! and interacting with the GitHub API. Authentication follows GitHub's official
//! documentation for GitHub Apps:
//!
//! 1. **App Authentication**: Generate a JWT signed with the app's private key
//!    to authenticate as the app itself. Used for listing installations.
//!
//! 2. **Installation Authentication**: Exchange the JWT for an installation
//!    access token to perform actions on behalf of an installation.

pub(crate) mod graphql;
pub(crate) mod oauth;
pub(crate) mod types;

pub use oauth::GitHubOAuthConfig;
pub use types::{Repository, WorkflowRun};
