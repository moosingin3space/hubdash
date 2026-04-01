//! Session management for authenticated users.

use std::future::Future;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Name of the session cookie.
pub const SESSION_COOKIE: &str = "hubdash_session";

/// Name of the OAuth state cookie used for CSRF protection.
pub const OAUTH_STATE_COOKIE: &str = "hubdash_oauth_state";

/// A unique session identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl Default for SessionId {
    fn default() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl SessionId {
    /// Creates a new random session ID.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the session ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// A GitHub user obtained from the OAuth flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    /// The user's GitHub ID.
    pub id: u64,
    /// The user's GitHub login name.
    pub login: String,
    /// The user's display name.
    pub name: Option<String>,
    /// URL to the user's avatar.
    pub avatar_url: Option<String>,
}

/// An authenticated user session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// The session ID.
    pub id: SessionId,
    /// The authenticated user.
    pub user: GitHubUser,
    /// The user's GitHub access token (stored server-side only).
    pub access_token: String,
}

/// Errors that can occur when interacting with the session store.
#[derive(Debug, Error)]
pub enum SessionStoreError {
    /// An internal error occurred.
    #[error("session store error: {0}")]
    Internal(String),
}

/// A session store for persisting user sessions.
pub trait SessionStore: Clone + Send + Sync + 'static {
    /// Stores a session.
    fn put(&self, session: Session) -> impl Future<Output = Result<(), SessionStoreError>> + Send;

    /// Retrieves a session by its ID.
    fn get(
        &self,
        id: &SessionId,
    ) -> impl Future<Output = Result<Option<Session>, SessionStoreError>> + Send;

    /// Deletes a session by its ID.
    fn delete(&self, id: &SessionId) -> impl Future<Output = Result<(), SessionStoreError>> + Send;
}
