//! Types for GitHub API responses.

use serde::Deserialize;

/// Represents an installation of the GitHub App.
#[derive(Debug, Clone, Deserialize)]
pub struct Installation {
    /// The unique identifier of the installation.
    pub id: u64,
    /// The account (user or organization) that owns the installation.
    pub account: InstallationAccount,
    /// The app ID associated with this installation.
    pub app_id: u64,
    /// Whether this installation has access to all repositories or only selected ones.
    pub repository_selection: RepositorySelection,
    /// URL to generate access tokens for this installation.
    pub access_tokens_url: String,
    /// URL to list repositories accessible to this installation.
    pub repositories_url: String,
}

/// The account (user or organization) that owns an installation.
#[derive(Debug, Clone, Deserialize)]
pub struct InstallationAccount {
    /// The login name of the account.
    pub login: String,
    /// The unique identifier of the account.
    pub id: u64,
    /// The type of account (User or Organization).
    #[serde(rename = "type")]
    pub account_type: AccountType,
}

/// Whether an installation has access to all or selected repositories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepositorySelection {
    /// Access to all repositories.
    All,
    /// Access to selected repositories only.
    Selected,
}

/// The type of GitHub account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum AccountType {
    /// A user account.
    User,
    /// An organization account.
    Organization,
}

/// An access token for a GitHub App installation.
#[derive(Debug, Clone, Deserialize)]
pub struct InstallationAccessToken {
    /// The access token string.
    pub token: String,
    /// When this token expires (ISO 8601 format).
    pub expires_at: String,
}
