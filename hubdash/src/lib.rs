//! This crate contains the Hubdash application,
//! which is a simple dashboard for managing your GitHub repositories.
//!
//! This is split into a library crate in order to enable deployment
//! in either a Cloudflare Worker or a standalone server.

use std::error::Error;
use std::future::Future;

use axum::{Router, middleware, routing::get};
use http_body::Body;

pub use axum;

mod assets;
mod auth_middleware;
mod auth_routes;
mod dashboard;
mod github;
mod landing;
mod layout;
mod mocks;
mod platform;
pub mod session;

#[cfg(test)]
mod sim_tests;

#[cfg(feature = "tokio")]
pub use platform::tokio::{InMemorySessionStore, TokioPlatform};

pub use github::GitHubOAuthConfig;
pub use session::SessionStore;

/// Shared application state.
#[derive(Clone)]
pub struct AppState<P: Platform> {
    /// The session store.
    pub sessions: <P as Platform>::SessionStore,
    /// GitHub OAuth configuration.
    pub oauth: GitHubOAuthConfig,
    /// The platform for creating HTTP clients.
    pub plat: P,
}

/// Creates an Axum router for the Hubdash application.
pub fn create_router<P: Platform>(plat: P, oauth: GitHubOAuthConfig) -> Router {
    let sessions = plat.create_session_store();
    let state = AppState::<P> {
        sessions,
        oauth,
        plat,
    };
    build_router(state)
}

/// Creates an Axum router from a pre-built [`AppState`].
///
/// This variant is useful in tests where the session store must be
/// pre-seeded before the server starts.
pub fn create_router_with_state<P: Platform>(state: AppState<P>) -> Router {
    build_router(state)
}

/// Internal helper that wires up all routes for a given `AppState`.
fn build_router<P: Platform>(state: AppState<P>) -> Router {
    let dashboard_routes = Router::new()
        .route("/", get(dashboard::dashboard_page))
        .route("/repo/{owner}/{repo}/expand", get(dashboard::repo_expand))
        .route("/repo/{owner}/{repo}/deps", get(dashboard::repo_deps))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware::require_auth::<P>,
        ));

    let router = Router::new()
        .route("/", get(landing::landing_page))
        .nest("/dashboard", dashboard_routes)
        .nest("/auth", auth_routes::router::<P>())
        .with_state(state)
        .nest("/assets", assets::router());

    #[cfg(debug_assertions)]
    let router = with_livereload(router);

    router
}

#[cfg(debug_assertions)]
fn with_livereload(router: Router) -> Router {
    router.layer(tower_livereload::LiveReloadLayer::new().request_predicate(
        |req: &http::Request<axum::body::Body>| !req.headers().contains_key("hx-request"),
    ))
}

/// Represents an HTTP Client interface.
///
/// Uses raw byte vectors for request and response bodies, leaving the
/// conversion to platform-specific body types to each platform implementation.
pub trait HttpClient: Send + Sync {
    /// The type of error this client can return.
    type Error: Error;

    /// Sends an HTTP request and returns the response.
    fn fetch<B>(
        &self,
        req: http::Request<B>,
    ) -> impl Future<
        Output = Result<http::Response<impl Body<Data = bytes::Bytes> + Send>, Self::Error>,
    > + Send
    where
        B: Body<Data = bytes::Bytes> + Send + 'static,
        B::Error: std::fmt::Display;
}

/// Represents a Platform that HubDash can run on.
pub trait Platform: Clone + Send + Sync + 'static {
    /// The type of HTTP client this platform supports.
    type HttpClient: HttpClient + 'static;

    /// The type of session store this platform supports.
    type SessionStore: SessionStore;

    /// Creates a new HTTP client for this platform.
    fn create_http_client(&self) -> Self::HttpClient;

    /// Creates a new session store for this platform.
    fn create_session_store(&self) -> Self::SessionStore;

    /// Returns the base URL of this deployment (e.g. `https://hubdash.example.com`).
    ///
    /// Used to build the OAuth `redirect_uri` sent to GitHub during sign-in.
    fn redirect_base_url(&self) -> url::Url;
}
