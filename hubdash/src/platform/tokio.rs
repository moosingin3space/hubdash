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

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::{HttpClient, Platform};
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};
use thiserror::Error;

/// A Tokio-based platform for running HubDash.
#[derive(Clone, Copy)]
pub struct TokioPlatform;

/// Internal body type used by the hyper client.
type HyperBody = http_body_util::Full<bytes::Bytes>;

/// An HTTP client backed by hyper-util's legacy client.
///
/// This wraps the [`hyper_util::client::legacy::Client`] which provides
/// automatic connection pooling and keep-alive management. See the module
/// documentation for why we use the legacy client over the composable pools.
pub struct HyperUtilHttpClient {
    client: Arc<Client<HttpConnector, HyperBody>>,
}

/// Error type for HTTP client operations.
#[derive(Debug, Error)]
pub enum HttpClientError {
    #[error(transparent)]
    Client(#[from] hyper_util::client::legacy::Error),
    #[error(transparent)]
    Body(#[from] hyper::Error),
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
    type Error = HttpClientError;

    async fn fetch(
        &self,
        req: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, Self::Error> {
        let client = self.client.clone();

        // Convert the Vec<u8> body to the hyper body type.
        let (parts, body) = req.into_parts();
        let hyper_req = http::Request::from_parts(parts, HyperBody::new(bytes::Bytes::from(body)));

        let resp = client
            .request(hyper_req)
            .await
            .map_err(HttpClientError::Client)?;
        let (parts, body) = resp.into_parts();
        let body_bytes = http_body_util::BodyExt::collect(body)
            .await
            .map_err(HttpClientError::Body)?
            .to_bytes();
        Ok(http::Response::from_parts(parts, body_bytes.to_vec()))
    }
}
