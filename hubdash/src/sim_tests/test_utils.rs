//! Shared test utilities for hubdash integration tests.

pub mod connector;
pub mod mock_github;
pub mod platform;

use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::session::{GitHubUser, Session, SessionId, SessionStore as _};
use crate::{
    AppState, GitHubOAuthConfig, InMemorySessionStore, create_router, create_router_with_state,
};
use http_body_util::{BodyExt, Full};
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use url::Url;

use connector::{SimConnector, sim_listen};
use mock_github::{API_GITHUB_HOST, mock_api_github_router, mock_github_router};
use platform::SimPlatform;

/// Port the hubdash app server listens on inside simulations.
pub const APP_PORT: u16 = 3000;
/// Port the mock GitHub server listens on inside simulations.
pub const GITHUB_PORT: u16 = 80;

/// Hostname for the hubdash app inside the turmoil simulation.
pub const APP_HOST: &str = "hubdash";
/// Hostname for the mock github.com inside the turmoil simulation.
pub const GITHUB_HOST: &str = "github.com";

/// Builds the OAuth config pointing to the in-simulation mock servers.
///
/// Uses plain HTTP so turmoil can intercept the connections without TLS.
pub fn test_oauth_config() -> GitHubOAuthConfig {
    GitHubOAuthConfig {
        client_id: "test-client-id".into(),
        client_secret: "test-client-secret".into(),
        github_base_url: Url::parse(&format!("http://{}:{}", GITHUB_HOST, GITHUB_PORT))
            .expect("valid test URL"),
        api_base_url: Url::parse(&format!("http://{}:{}", API_GITHUB_HOST, GITHUB_PORT))
            .expect("valid test URL"),
    }
}

// ── server registration helpers ──────────────────────────────────────────────

fn register_app_server(sim: &mut turmoil::Sim<'_>) {
    sim.host(APP_HOST, || async {
        let listener = sim_listen(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), APP_PORT))
            .await
            .unwrap();
        let router = create_router(SimPlatform, test_oauth_config());
        axum::serve(listener, router).await.unwrap();
        Ok(())
    });
}

fn register_app_server_with_session(sim: &mut turmoil::Sim<'_>, session_id: SessionId) {
    sim.host(APP_HOST, move || {
        let session_id = session_id.clone();
        async move {
            let sessions = InMemorySessionStore::new();
            sessions
                .put(Session {
                    id: session_id.clone(),
                    user: GitHubUser {
                        id: 1,
                        login: "test-user".into(),
                        name: Some("Test User".into()),
                        avatar_url: None,
                    },
                    access_token: "fake-access-token".into(),
                })
                .await
                .unwrap();
            let state = AppState::<SimPlatform> {
                sessions,
                oauth: test_oauth_config(),
                plat: SimPlatform,
            };
            let router = create_router_with_state(state);
            let listener = sim_listen(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), APP_PORT))
                .await
                .unwrap();
            axum::serve(listener, router).await.unwrap();
            Ok(())
        }
    });
}

fn register_github_server(sim: &mut turmoil::Sim<'_>) {
    sim.host(GITHUB_HOST, || async {
        let listener = sim_listen(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            GITHUB_PORT,
        ))
        .await
        .unwrap();
        axum::serve(listener, mock_github_router()).await.unwrap();
        Ok(())
    });
}

fn register_api_github_server(sim: &mut turmoil::Sim<'_>) {
    sim.host(API_GITHUB_HOST, || async {
        let listener = sim_listen(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            GITHUB_PORT,
        ))
        .await
        .unwrap();
        axum::serve(listener, mock_api_github_router())
            .await
            .unwrap();
        Ok(())
    });
}

// ── simulation runners ───────────────────────────────────────────────────────

/// Runs a turmoil simulation with the hubdash app server, then calls
/// `client_fn` in the simulated client host.
pub fn run_sim<F, Fut>(client_fn: F)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<(), Box<dyn std::error::Error>>> + 'static,
{
    let mut sim = turmoil::Builder::new().build();
    register_app_server(&mut sim);
    sim.client("client", client_fn());
    sim.run().unwrap();
}

/// Runs a turmoil simulation with the hubdash app server and both GitHub mock
/// servers (`github.com` for token exchange, `api.github.com` for user info),
/// then calls `client_fn` in the simulated client host.
pub fn run_github_sim<F, Fut>(client_fn: F)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<(), Box<dyn std::error::Error>>> + 'static,
{
    let mut sim = turmoil::Builder::new().build();
    register_app_server(&mut sim);
    register_github_server(&mut sim);
    register_api_github_server(&mut sim);
    sim.client("client", client_fn());
    sim.run().unwrap();
}

/// Runs a turmoil simulation with the hubdash app server pre-seeded with a
/// valid session, then calls `client_fn` with the ready-to-use `Cookie`
/// header value for that session.
pub fn run_authed_sim<F, Fut>(client_fn: F)
where
    F: FnOnce(String) -> Fut,
    Fut: Future<Output = Result<(), Box<dyn std::error::Error>>> + 'static,
{
    let session_id = SessionId::new();
    let cookie = format!("hubdash_session={}", session_id.as_str());
    let mut sim = turmoil::Builder::new().build();
    register_app_server_with_session(&mut sim, session_id);
    register_api_github_server(&mut sim);
    sim.client("client", client_fn(cookie));
    sim.run().unwrap();
}

// ── HTTP helpers ─────────────────────────────────────────────────────────────

/// Sends a GET request to the hubdash app and returns `(status, body_string)`.
pub async fn get(path: &str) -> (http::StatusCode, String) {
    get_with_cookie(path, None).await
}

/// Sends a GET request with an optional `Cookie` header value.
pub async fn get_with_cookie(path: &str, cookie: Option<&str>) -> (http::StatusCode, String) {
    let (status, _headers, body) = request_with_cookie(path, cookie).await;
    (status, body)
}

/// Sends a GET request and returns `(status, response_headers, body_string)`.
///
/// Use this when you need to inspect `Set-Cookie` or other response headers.
pub async fn request_with_cookie(
    path: &str,
    cookie: Option<&str>,
) -> (http::StatusCode, http::HeaderMap, String) {
    let client: Client<SimConnector, Full<bytes::Bytes>> =
        Client::builder(TokioExecutor::new()).build(SimConnector);

    let uri = format!("http://{}:{}{}", APP_HOST, APP_PORT, path);

    let mut builder = http::Request::builder().method(http::Method::GET).uri(&uri);

    if let Some(c) = cookie {
        builder = builder.header(http::header::COOKIE, c);
    }

    let req = builder.body(Full::new(bytes::Bytes::new())).unwrap();

    let resp = client.request(req).await.unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body).into_owned();
    (status, headers, body_str)
}
