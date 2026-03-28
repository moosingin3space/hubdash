//! A turmoil-compatible `Platform` implementation for integration tests.
//!
//! `SimPlatform` wires the `HttpClient` trait to a hyper legacy client that
//! routes TCP through turmoil's simulated network, and provides the same
//! `InMemorySessionStore` used by the real Tokio platform.

use std::sync::Arc;

use crate::InMemorySessionStore;
use crate::{HttpClient, Platform};
use http_body::Body;
use http_body_util::{BodyExt, Full};
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use thiserror::Error;
use url::Url;

use super::connector::SimConnector;
use super::{APP_HOST, APP_PORT};

/// Error type for the simulated HTTP client.
#[derive(Debug, Error)]
pub enum SimHttpClientError {
    #[error("hyper error: {0}")]
    Hyper(#[from] hyper_util::client::legacy::Error),
    #[error("body error: {0}")]
    Body(String),
}

/// HTTP client that routes through turmoil's simulated TCP stack.
#[derive(Clone)]
pub struct SimHttpClient {
    inner: Arc<Client<SimConnector, Full<bytes::Bytes>>>,
}

impl SimHttpClient {
    /// Creates a new `SimHttpClient`.
    pub fn new() -> Self {
        let client = Client::builder(TokioExecutor::new()).build(SimConnector);
        Self {
            inner: Arc::new(client),
        }
    }
}

impl HttpClient for SimHttpClient {
    type Error = SimHttpClientError;

    async fn fetch<B>(
        &self,
        req: http::Request<B>,
    ) -> Result<http::Response<impl Body<Data = bytes::Bytes> + Send>, Self::Error>
    where
        B: Body<Data = bytes::Bytes> + Send + 'static,
        B::Error: std::fmt::Display,
    {
        let (parts, body) = req.into_parts();
        let body_bytes = body
            .collect()
            .await
            .map_err(|e| SimHttpClientError::Body(e.to_string()))?
            .to_bytes();
        let hyper_req = http::Request::from_parts(parts, Full::new(body_bytes));

        let resp = self
            .inner
            .request(hyper_req)
            .await
            .map_err(SimHttpClientError::Hyper)?;
        let (parts, body) = resp.into_parts();
        let collected = body
            .collect()
            .await
            .map_err(|e| SimHttpClientError::Body(e.to_string()))?;
        Ok(http::Response::from_parts(parts, collected))
    }
}

/// A `Platform` for turmoil-based integration tests.
#[derive(Clone)]
pub struct SimPlatform;

impl Platform for SimPlatform {
    type HttpClient = SimHttpClient;
    type SessionStore = InMemorySessionStore;

    fn redirect_base_url(&self) -> Url {
        Url::parse(&format!("http://{}:{}", APP_HOST, APP_PORT)).expect("valid sim base URL")
    }

    fn create_http_client(&self) -> Self::HttpClient {
        SimHttpClient::new()
    }

    fn create_session_store(&self) -> Self::SessionStore {
        InMemorySessionStore::new()
    }
}
