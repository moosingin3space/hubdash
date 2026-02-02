//! A Tokio implementation of a platform.
//!
//! # Why the Legacy Client?
//!
//! This implementation uses [`hyper_util::client::legacy::Client`] rather than
//! building a custom pool stack with the new composable pools
//! ([`hyper_util::client::pool`]).
//!
//! The legacy client provides a batteries-included HTTP client with built-in
//! connection pooling, HTTP/1.1 and HTTP/2 support, and automatic keep-alive
//! management. It internally uses the same pool primitives that power the new
//! composable pools API.
//!
//! The new composable pools (`pool::cache`, `pool::singleton`, `pool::negotiate`,
//! `pool::map`) are designed for advanced use cases where you need fine-grained
//! control over connection management—such as custom pooling keys, connection
//! limits, or protocol-specific pooling strategies. For most applications,
//! the legacy client is the recommended starting point.
//!
//! If we need custom pooling behavior in the future (e.g., per-host connection
//! limits or custom expiration logic), we can migrate to the composable pools
//! API while keeping the same `HttpClient` trait interface.

use std::sync::Arc;

use crate::{HttpClient, Platform};
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};

/// A Tokio-based platform for running HubDash.
pub struct TokioPlatform;

/// Body type for HTTP requests and responses.
pub type Body = http_body_util::Full<bytes::Bytes>;

/// An HTTP client backed by hyper-util's legacy client.
///
/// This wraps the [`hyper_util::client::legacy::Client`] which provides
/// automatic connection pooling and keep-alive management. See the module
/// documentation for why we use the legacy client over the composable pools.
pub struct HyperUtilHttpClient {
    client: Arc<Client<HttpConnector, Body>>,
}

/// Error type for HTTP client operations.
#[derive(Debug)]
pub enum HttpClientError {
    /// Error from the legacy client.
    Client(hyper_util::client::legacy::Error),
    /// Error collecting response body.
    Body(hyper::Error),
}

impl std::fmt::Display for HttpClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Client(e) => write!(f, "{e}"),
            Self::Body(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for HttpClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Client(e) => Some(e),
            Self::Body(e) => Some(e),
        }
    }
}

impl Platform for TokioPlatform {
    type HttpClient = HyperUtilHttpClient;

    fn create_http_client(&self) -> Self::HttpClient {
        let connector = HttpConnector::new();
        let client = Client::builder(TokioExecutor::new()).build(connector);
        HyperUtilHttpClient {
            client: Arc::new(client),
        }
    }
}

impl HttpClient for HyperUtilHttpClient {
    type Body = Body;

    type Error = HttpClientError;

    fn fetch(
        &self,
        req: http::Request<Self::Body>,
    ) -> Result<http::Response<Self::Body>, Self::Error> {
        let client = self.client.clone();
        let handle = tokio::runtime::Handle::current();
        handle.block_on(async move {
            let resp = client.request(req).await.map_err(HttpClientError::Client)?;
            let (parts, body) = resp.into_parts();
            let body_bytes = http_body_util::BodyExt::collect(body)
                .await
                .map_err(HttpClientError::Body)?
                .to_bytes();
            Ok(http::Response::from_parts(
                parts,
                http_body_util::Full::new(body_bytes),
            ))
        })
    }
}
