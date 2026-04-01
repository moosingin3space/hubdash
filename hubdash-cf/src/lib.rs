//! HubDash as a Cloudflare Worker

use tracing_subscriber::prelude::*;
use tracing_web::MakeConsoleWriter;

use std::sync::Arc;

use http_body_util::BodyExt;
use tower_service::Service;
use wasm_bindgen::JsValue;
use worker::*;

use hubdash::session::{Session, SessionId, SessionStore, SessionStoreError};
use hubdash::{GitHubOAuthConfig, HttpClient, Platform};

#[derive(Clone)]
struct CloudflarePlatform {
    base_url: url::Url,
    db: Arc<worker::d1::D1Database>,
}

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
struct CloudflareSessionStore(Arc<worker::d1::D1Database>);

/// A row returned from the sessions table.
#[derive(serde::Deserialize)]
struct SessionRow {
    data: String,
}

impl SessionStore for CloudflareSessionStore {
    #[worker::send]
    async fn put(&self, session: Session) -> Result<(), SessionStoreError> {
        let data = serde_json::to_string(&session)
            .map_err(|e| SessionStoreError::Internal(e.to_string()))?;
        let expires_at = js_sys::Date::now() as u64 / 1000 + 604_800; // 7 days, matching cookie max_age
        let stmt = self
            .0
            .prepare(
                "INSERT OR REPLACE INTO sessions (id, data, expires_at) VALUES (?1, json(?2), ?3)",
            )
            .bind(&[
                JsValue::from_str(session.id.as_str()),
                JsValue::from_str(&data),
                JsValue::from_f64(expires_at as f64),
            ])
            .map_err(|e: worker::Error| SessionStoreError::Internal(e.to_string()))?;
        stmt.run()
            .await
            .map_err(|e: worker::Error| SessionStoreError::Internal(e.to_string()))?;
        Ok(())
    }

    #[worker::send]
    async fn get(&self, id: &SessionId) -> Result<Option<Session>, SessionStoreError> {
        let now = js_sys::Date::now() as u64 / 1000;
        let stmt = self
            .0
            .prepare("SELECT json(data) AS data FROM sessions WHERE id = ?1 AND expires_at > ?2")
            .bind(&[
                JsValue::from_str(id.as_str()),
                JsValue::from_f64(now as f64),
            ])
            .map_err(|e: worker::Error| SessionStoreError::Internal(e.to_string()))?;
        let row = stmt
            .first::<SessionRow>(None)
            .await
            .map_err(|e: worker::Error| SessionStoreError::Internal(e.to_string()))?;
        match row {
            None => Ok(None),
            Some(r) => {
                let session: Session = serde_json::from_str(&r.data)
                    .map_err(|e| SessionStoreError::Internal(e.to_string()))?;
                Ok(Some(session))
            }
        }
    }

    #[worker::send]
    async fn delete(&self, id: &SessionId) -> Result<(), SessionStoreError> {
        let stmt = self
            .0
            .prepare("DELETE FROM sessions WHERE id = ?1")
            .bind(&[JsValue::from_str(id.as_str())])
            .map_err(|e: worker::Error| SessionStoreError::Internal(e.to_string()))?;
        stmt.run()
            .await
            .map_err(|e: worker::Error| SessionStoreError::Internal(e.to_string()))?;
        Ok(())
    }
}

impl Platform for CloudflarePlatform {
    type HttpClient = CloudflareHttpClient;
    type SessionStore = CloudflareSessionStore;

    fn redirect_base_url(&self) -> url::Url {
        self.base_url.clone()
    }

    fn create_http_client(&self) -> Self::HttpClient {
        CloudflareHttpClient
    }

    fn create_session_store(&self) -> Self::SessionStore {
        CloudflareSessionStore(Arc::clone(&self.db))
    }
}

/// Derives the base URL (scheme + host, no path) from an incoming HTTP request.
fn base_url_from_request(req: &HttpRequest) -> url::Url {
    let uri = req.uri();
    let scheme = uri.scheme_str().unwrap_or("https");
    let host = uri.host().unwrap_or("localhost");
    match uri.port_u16() {
        Some(port) => {
            url::Url::parse(&format!("{scheme}://{host}:{port}")).expect("derived URL is valid")
        }
        None => url::Url::parse(&format!("{scheme}://{host}")).expect("derived URL is valid"),
    }
}

#[event(start)]
fn start() {
    console_error_panic_hook::set_once();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .without_time()
        .with_writer(MakeConsoleWriter);

    tracing_subscriber::registry().with(fmt_layer).init();
}

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    ctx: Context,
) -> Result<http::Response<hubdash::axum::body::Body>> {
    let _ = ctx;

    let base_url = base_url_from_request(&req);
    let oauth = GitHubOAuthConfig {
        client_id: env.var("GITHUB_CLIENT_ID")?.to_string(),
        client_secret: env.var("GITHUB_CLIENT_SECRET")?.to_string(),
        ..Default::default()
    };
    let db = Arc::new(env.d1("DB")?);

    let mut router = hubdash::create_router(CloudflarePlatform { base_url, db }, oauth);

    Ok(router.call(req).await?)
}

#[event(scheduled)]
async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    let Ok(db) = env.d1("DB") else {
        tracing::error!("session cleanup: DB binding unavailable");
        return;
    };
    let now = js_sys::Date::now() as u64 / 1000;
    let result = db
        .prepare("DELETE FROM sessions WHERE expires_at <= ?1")
        .bind(&[JsValue::from_f64(now as f64)]);
    let Ok(stmt) = result else {
        tracing::error!("session cleanup: failed to bind statement");
        return;
    };
    if let Err(e) = stmt.run().await {
        tracing::error!("session cleanup failed: {e}");
    }
}
