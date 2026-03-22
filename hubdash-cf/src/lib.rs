//! HubDash as a Cloudflare Worker

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use http_body_util::BodyExt;
use tower_service::Service;
use worker::*;

use hubdash::session::{Session, SessionId, SessionStore, SessionStoreError};
use hubdash::{GitHubOAuthConfig, HttpClient, Platform};

#[derive(Clone, Copy)]
struct CloudflarePlatform;

#[derive(Clone, Copy)]
struct CloudflareHttpClient;

impl HttpClient for CloudflareHttpClient {
    type Error = worker::Error;

    #[worker::send]
    async fn fetch<B>(
        &self,
        req: http::Request<B>,
    ) -> Result<http::Response<impl http_body::Body<Data = bytes::Bytes> + Send>, Self::Error>
    where
        B: http_body::Body<Data = bytes::Bytes> + Send + 'static,
        B::Error: std::fmt::Display,
    {
        let worker_req = worker::Request::try_from(req)?;
        let worker_resp = worker::Fetch::Request(worker_req).send().await?;
        let (parts, body) = worker::HttpResponse::try_from(worker_resp)?.into_parts();
        let body_bytes = body
            .collect()
            .await
            .map_err(|e| worker::Error::RustError(e.to_string()))?
            .to_bytes();
        Ok(http::Response::from_parts(
            parts,
            http_body_util::Full::new(body_bytes),
        ))
    }
}

#[derive(Clone)]
struct CloudflareSessionStore(Arc<Mutex<HashMap<SessionId, Session>>>);

impl CloudflareSessionStore {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }
}

impl SessionStore for CloudflareSessionStore {
    fn put(&self, session: Session) -> Result<(), SessionStoreError> {
        self.0
            .lock()
            .map_err(|e| SessionStoreError::Internal(e.to_string()))?
            .insert(session.id.clone(), session);
        Ok(())
    }

    fn get(&self, id: &SessionId) -> Result<Option<Session>, SessionStoreError> {
        Ok(self
            .0
            .lock()
            .map_err(|e| SessionStoreError::Internal(e.to_string()))?
            .get(id)
            .cloned())
    }

    fn delete(&self, id: &SessionId) -> Result<(), SessionStoreError> {
        self.0
            .lock()
            .map_err(|e| SessionStoreError::Internal(e.to_string()))?
            .remove(id);
        Ok(())
    }
}

impl Platform for CloudflarePlatform {
    type HttpClient = CloudflareHttpClient;
    type SessionStore = CloudflareSessionStore;

    fn create_http_client(&self) -> Self::HttpClient {
        CloudflareHttpClient
    }

    fn create_session_store(&self) -> Self::SessionStore {
        CloudflareSessionStore::new()
    }
}

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    ctx: Context,
) -> Result<http::Response<hubdash::axum::body::Body>> {
    console_error_panic_hook::set_once();
    let _ = ctx;

    let oauth = GitHubOAuthConfig {
        client_id: env.var("GITHUB_CLIENT_ID")?.to_string(),
        client_secret: env.var("GITHUB_CLIENT_SECRET")?.to_string(),
        ..Default::default()
    };

    let mut router = hubdash::create_router(CloudflarePlatform, oauth);

    Ok(router.call(req).await?)
}
