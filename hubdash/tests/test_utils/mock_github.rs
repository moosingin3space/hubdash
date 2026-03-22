//! A minimal mock GitHub OAuth server running inside turmoil.
#![allow(dead_code)]
//!
//! # Design
//!
//! The mock server listens at `github.com:80` (turmoil resolves hostnames to
//! simulated IPs). It handles two endpoints used by the OAuth flow:
//!
//! - `POST /login/oauth/access_token` – returns a canned access token
//! - `GET  /user` (via `api.github.com`) – returns canned user JSON
//!
//! To keep things simple both hostnames resolve to the same turmoil host; we
//! differentiate by path only.
//!
//! # Usage
//!
//! Call [`mock_github_router`] and pass the returned `Router` to
//! `axum::serve` on the desired simulated address.

use axum::{Router, routing::{get, post}};
use axum::response::IntoResponse;
use http::StatusCode;

/// The canned access token the mock server issues.
pub const MOCK_ACCESS_TOKEN: &str = "mock-access-token-123";

/// The canned user login the mock server returns.
pub const MOCK_USER_LOGIN: &str = "test-user";

/// The canned GitHub user ID.
pub const MOCK_USER_ID: u64 = 42;

/// Creates the mock GitHub API/OAuth router.
pub fn mock_github_router() -> Router {
    Router::new()
        .route("/login/oauth/access_token", post(mock_token_endpoint))
        .route("/user", get(mock_user_endpoint))
}

/// Returns a successful token exchange response.
async fn mock_token_endpoint() -> impl IntoResponse {
    let body = serde_json::json!({
        "access_token": MOCK_ACCESS_TOKEN,
        "token_type": "bearer",
        "scope": "read:user"
    });
    (
        StatusCode::OK,
        [("content-type", "application/json")],
        axum::Json(body),
    )
}

/// Returns a GitHub user object.
async fn mock_user_endpoint() -> impl IntoResponse {
    let body = serde_json::json!({
        "id": MOCK_USER_ID,
        "login": MOCK_USER_LOGIN,
        "name": "Test User",
        "avatar_url": "https://github.com/identicons/test-user.png"
    });
    (
        StatusCode::OK,
        [("content-type", "application/json")],
        axum::Json(body),
    )
}
