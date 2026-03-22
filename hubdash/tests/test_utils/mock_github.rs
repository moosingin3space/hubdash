//! Minimal mock GitHub OAuth servers running inside turmoil.
//!
//! # Design
//!
//! Two separate Axum routers mimic the two GitHub hostnames used by the OAuth
//! flow:
//!
//! - **`github.com`** (via [`mock_github_router`]):
//!   `POST /login/oauth/access_token` — returns [`MOCK_ACCESS_TOKEN`]
//!
//! - **`api.github.com`** (via [`mock_api_github_router`]):
//!   `GET /user` — returns a canned [`GitHubUser`] payload
//!
//! Both are plain HTTP so turmoil can intercept the connections without TLS.
//!
//! # Usage
//!
//! ```ignore
//! register_github_server(&mut sim);     // github.com:80
//! register_api_github_server(&mut sim); // api.github.com:80
//! ```

use axum::response::IntoResponse;
use axum::{
    Router,
    routing::{get, post},
};
use http::StatusCode;

/// The canned access token the mock token endpoint issues.
pub const MOCK_ACCESS_TOKEN: &str = "mock-access-token-123";

/// The canned GitHub user login.
pub const MOCK_USER_LOGIN: &str = "test-user";

/// The canned GitHub user ID.
pub const MOCK_USER_ID: u64 = 42;

/// Hostname for the mock `api.github.com` used inside turmoil.
pub const API_GITHUB_HOST: &str = "api.github.com";

// ── github.com router ────────────────────────────────────────────────────────

/// Router for the mock `github.com` host.
///
/// Handles `POST /login/oauth/access_token`.
pub fn mock_github_router() -> Router {
    Router::new().route("/login/oauth/access_token", post(mock_token_endpoint))
}

/// Returns a successful token exchange response regardless of the posted code.
async fn mock_token_endpoint() -> impl IntoResponse {
    let body = serde_json::json!({
        "access_token": MOCK_ACCESS_TOKEN,
        "token_type": "bearer",
        "scope": "read:user"
    });
    (StatusCode::OK, axum::Json(body))
}

// ── api.github.com router ────────────────────────────────────────────────────

/// Router for the mock `api.github.com` host.
///
/// Handles `GET /user`.
pub fn mock_api_github_router() -> Router {
    Router::new().route("/user", get(mock_user_endpoint))
}

/// Returns a canned GitHub user object.
async fn mock_user_endpoint() -> impl IntoResponse {
    let body = serde_json::json!({
        "id": MOCK_USER_ID,
        "login": MOCK_USER_LOGIN,
        "name": "Test User",
        "avatar_url": null
    });
    (StatusCode::OK, axum::Json(body))
}
