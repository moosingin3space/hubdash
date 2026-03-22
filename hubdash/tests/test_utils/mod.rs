//! Shared test utilities for hubdash integration tests.
#![allow(dead_code)]

pub mod connector;
pub mod mock_github;
pub mod platform;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use http_body_util::{BodyExt, Full};
use hubdash::{GitHubOAuthConfig, create_router};
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;

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
        github_base_url: format!("http://{}:{}", GITHUB_HOST, GITHUB_PORT),
        api_base_url: format!("http://{}:{}", API_GITHUB_HOST, GITHUB_PORT),
    }
}

/// Registers the hubdash app server host in a turmoil simulation.
pub fn register_app_server(sim: &mut turmoil::Sim<'_>) {
    sim.host(APP_HOST, || async {
        let listener = sim_listen(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            APP_PORT,
        ))
        .await
        .unwrap();
        let router = create_router(SimPlatform, test_oauth_config());
        axum::serve(listener, router).await.unwrap();
        Ok(())
    });
}

/// Registers a hubdash app server with a pre-built OAuth config.
pub fn register_app_server_with_config(
    sim: &mut turmoil::Sim<'_>,
    oauth: GitHubOAuthConfig,
) {
    sim.host(APP_HOST, move || {
        let oauth = oauth.clone();
        async move {
            let listener = sim_listen(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                APP_PORT,
            ))
            .await
            .unwrap();
            let router = create_router(SimPlatform, oauth);
            axum::serve(listener, router).await.unwrap();
            Ok(())
        }
    });
}

/// Registers the mock `github.com` host (OAuth token endpoint).
pub fn register_github_server(sim: &mut turmoil::Sim<'_>) {
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

/// Registers the mock `api.github.com` host (user info endpoint).
pub fn register_api_github_server(sim: &mut turmoil::Sim<'_>) {
    sim.host(API_GITHUB_HOST, || async {
        let listener = sim_listen(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            GITHUB_PORT,
        ))
        .await
        .unwrap();
        axum::serve(listener, mock_api_github_router()).await.unwrap();
        Ok(())
    });
}

/// Sends a GET request to the hubdash app and returns `(status, body_string)`.
pub async fn get(path: &str) -> (http::StatusCode, String) {
    get_with_cookie(path, None).await
}

/// Sends a GET request with an optional `Cookie` header value.
pub async fn get_with_cookie(
    path: &str,
    cookie: Option<&str>,
) -> (http::StatusCode, String) {
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

    let mut builder = http::Request::builder()
        .method(http::Method::GET)
        .uri(&uri);

    if let Some(c) = cookie {
        builder = builder.header(http::header::COOKIE, c);
    }

    let req = builder.body(Full::new(bytes::Bytes::new())).unwrap();

    let resp = client.request(req).await.unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let body = resp
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let body_str = String::from_utf8_lossy(&body).into_owned();
    (status, headers, body_str)
}
