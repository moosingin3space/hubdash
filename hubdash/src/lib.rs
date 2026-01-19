//! This crate contains the Hubdash application,
//! which is a simple dashboard for managing your GitHub repositories.
//!
//! This is split into a library crate in order to enable deployment
//! in either a Cloudflare Worker or a standalone server.

use axum::{Router, routing::get};

mod assets;
mod dashboard;
mod github;
mod landing;
mod layout;
mod mocks;

/// Creates an Axum router for the Hubdash application.
pub fn create_router() -> Router {
    Router::new()
        .route("/", get(landing::landing_page))
        .route("/dashboard", get(dashboard::dashboard_page))
        .route(
            "/dashboard/repo/{owner}/{repo}/expand",
            get(dashboard::repo_expand),
        )
        .route(
            "/dashboard/repo/{owner}/{repo}/deps",
            get(dashboard::repo_deps),
        )
        .nest("/assets", assets::router())
}
