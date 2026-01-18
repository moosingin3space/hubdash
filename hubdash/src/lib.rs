//! This crate contains the Hubdash application,
//! which is a simple dashboard for managing your GitHub repositories.
//!
//! This is split into a library crate in order to enable deployment
//! in either a Cloudflare Worker or a standalone server.

use axum::Router;

/// Creates an Axum router for the Hubdash application.
pub fn create_router() -> Router {
    Router::new()
}
