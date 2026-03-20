//! Authentication middleware that protects routes requiring a valid session.

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;

use crate::session::{SESSION_COOKIE, SessionId, SessionStore};
use crate::{AppState, Platform};

/// Middleware that requires a valid session cookie.
///
/// If the session cookie is missing or invalid, redirects to the landing page.
/// If valid, the session is inserted into request extensions for downstream handlers.
pub async fn require_auth<P: Platform>(
    State(state): State<AppState<P>>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Response {
    let session_id = jar
        .get(SESSION_COOKIE)
        .map(|c| SessionId::from(c.value().to_owned()));

    let session = session_id.and_then(|id| state.sessions.get(&id).ok().flatten());

    match session {
        Some(session) => {
            req.extensions_mut().insert(session);
            next.run(req).await
        }
        None => Redirect::to("/").into_response(),
    }
}
