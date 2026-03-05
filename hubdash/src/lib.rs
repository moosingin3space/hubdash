//! This crate contains the Hubdash application,
//! which is a simple dashboard for managing your GitHub repositories.
//!
//! This is split into a library crate in order to enable deployment
//! in either a Cloudflare Worker or a standalone server.

use std::error::Error;

use axum::{Router, routing::get};

mod assets;
mod dashboard;
mod github;
mod landing;
mod layout;
mod mocks;
mod platform;

#[cfg(feature = "tokio")]
pub use platform::tokio::TokioPlatform;

/// Creates an Axum router for the Hubdash application.
pub fn create_router(plat: impl Platform) -> Router {
    // TODO use the platform
    let _plat = plat;
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

/// Represents an HTTP Client interface.
pub trait HttpClient {
    /// The type of error this platform can return.
    type Error: Error;

    /// Fetches
    fn fetch(
        &self,
        req: http::Request<Vec<u8>>,
    ) -> impl Future<Output = Result<http::Response<Vec<u8>>, Self::Error>> + Send;
}

/// Represents a Platform that HubDash can run on.
pub trait Platform {
    /// The type of HTTP client this platform supports.
    type HttpClient: HttpClient;

    /// Creates a new HTTP client for this platform.
    fn create_http_client(&self) -> Self::HttpClient;
}
