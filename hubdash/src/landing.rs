//! Landing page for the Hubdash application.

use axum::extract::State;
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use maud::html;

use crate::layout::base_layout;
use crate::session::{SESSION_COOKIE, SessionId, SessionStore};
use crate::{AppState, Platform};

/// Renders the landing page, or redirects to the dashboard if a session exists.
pub async fn landing_page<P: Platform>(
    State(state): State<AppState<P>>,
    jar: CookieJar,
) -> Response {
    let session_id = jar
        .get(SESSION_COOKIE)
        .map(|c| SessionId::from(c.value().to_owned()));

    let has_session = match session_id {
        Some(id) => state.sessions.get(&id).await.ok().flatten().is_some(),
        None => false,
    };

    if has_session {
        return Redirect::to("/dashboard").into_response();
    }

    let body = html! {
        div class="container" {
            h1 { "Hubdash" }
            p { "Monitor your GitHub repositories' CI/CD pipeline health and dependency freshness in one place." }
            a href="/auth/signin" class="sign-in-btn" { "Sign in with GitHub" }
        }
    };

    base_layout(
        "Hubdash",
        &["/assets/theme.css", "/assets/landing.css"],
        &[],
        body,
    )
    .into_response()
}
