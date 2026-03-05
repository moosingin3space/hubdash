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

#[allow(dead_code)]
mod auth;
#[allow(dead_code)]
mod client;
pub(crate) mod oauth;
#[allow(dead_code)]
mod types;

#[allow(unused_imports)]
pub use auth::{AppCredentials, AppCredentialsError};
#[allow(unused_imports)]
pub use client::{GitHubClient, GitHubClientError};
pub use oauth::GitHubOAuthConfig;
#[allow(unused_imports)]
pub use types::{Installation, InstallationAccessToken, InstallationAccount};
