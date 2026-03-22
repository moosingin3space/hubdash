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

use crate::session::{Session, SessionId, SessionStore, SessionStoreError};
use crate::{HttpClient, Platform};
use http_body::Body;
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use thiserror::Error;

/// A Tokio-based platform for running HubDash.
#[derive(Clone, Copy)]
pub struct TokioPlatform;

/// Internal body type used by the hyper client.
type HyperBody = http_body_util::Full<bytes::Bytes>;

/// Internal connector type that supports HTTPS via rustls.
type HttpsConnector =
    hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>;

/// An HTTP client backed by hyper-util's legacy client.
///
/// This wraps the [`hyper_util::client::legacy::Client`] which provides
/// automatic connection pooling and keep-alive management. See the module
/// documentation for why we use the legacy client over the composable pools.
///
/// Uses hyper-rustls for HTTPS support with native TLS verification.
pub struct HyperUtilHttpClient {
    client: Arc<Client<HttpsConnector, HyperBody>>,
}

/// Error type for HTTP client operations.
#[derive(Debug, Error)]
pub enum HttpClientError {
    #[error(transparent)]
    Client(#[from] hyper_util::client::legacy::Error),
    #[error("{0}")]
    Body(String),
}

/// An in-memory session store backed by a `HashMap`.
#[derive(Clone)]
pub struct InMemorySessionStore {
    sessions: Arc<Mutex<HashMap<SessionId, Session>>>,
}

impl InMemorySessionStore {
    /// Creates a new empty in-memory session store.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionStore for InMemorySessionStore {
    fn put(&self, session: Session) -> Result<(), SessionStoreError> {
        let mut store = self
            .sessions
            .lock()
            .map_err(|e| SessionStoreError::Internal(e.to_string()))?;
        store.insert(session.id.clone(), session);
        Ok(())
    }

    fn get(&self, id: &SessionId) -> Result<Option<Session>, SessionStoreError> {
        let store = self
            .sessions
            .lock()
            .map_err(|e| SessionStoreError::Internal(e.to_string()))?;
        Ok(store.get(id).cloned())
    }

    fn delete(&self, id: &SessionId) -> Result<(), SessionStoreError> {
        let mut store = self
            .sessions
            .lock()
            .map_err(|e| SessionStoreError::Internal(e.to_string()))?;
        store.remove(id);
        Ok(())
    }
}

impl Platform for TokioPlatform {
    type HttpClient = HyperUtilHttpClient;
    type SessionStore = InMemorySessionStore;

    fn create_http_client(&self) -> Self::HttpClient {
        let https = HttpsConnectorBuilder::new()
            .with_native_roots()
            .expect("failed to load native roots for TLS verification")
            .https_or_http()
            .enable_all_versions()
            .build();
        let client = Client::builder(TokioExecutor::new()).build(https);
        HyperUtilHttpClient {
            client: Arc::new(client),
        }
    }

    fn create_session_store(&self) -> Self::SessionStore {
        InMemorySessionStore::new()
    }
}

impl HttpClient for HyperUtilHttpClient {
    type Error = HttpClientError;

    async fn fetch<B>(
        &self,
        req: http::Request<B>,
    ) -> Result<http::Response<impl Body<Data = bytes::Bytes>>, Self::Error>
    where
        B: Body<Data = bytes::Bytes> + Send + 'static,
        B::Error: std::fmt::Display,
    {
        let client = self.client.clone();

        // Convert the body to the hyper body type.
        let (parts, body) = req.into_parts();
        let hyper_body = HyperBody::new(
            http_body_util::BodyExt::collect(body)
                .await
                .map_err(|e| HttpClientError::Body(e.to_string()))?
                .to_bytes(),
        );
        let hyper_req = http::Request::from_parts(parts, hyper_body);

        let resp = client
            .request(hyper_req)
            .await
            .map_err(HttpClientError::Client)?;
        let (parts, body) = resp.into_parts();
        let body_bytes = http_body_util::BodyExt::collect(body)
            .await
            .map_err(|e| HttpClientError::Body(e.to_string()))?;
        Ok(http::Response::from_parts(parts, body_bytes))
    }
}
