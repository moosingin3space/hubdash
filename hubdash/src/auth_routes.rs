//! Authentication routes for GitHub OAuth sign-in.

use axum::extract::State;
use axum::response::{IntoResponse, Redirect};
use axum::routing::get;
use axum::{Router, extract::Query};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::Cookie;
use serde::Deserialize;
use tracing::error;

use crate::github::oauth;
use crate::session::{OAUTH_STATE_COOKIE, SESSION_COOKIE, Session, SessionId, SessionStore as _};
use crate::{AppState, Platform};

/// Query parameters from the GitHub OAuth callback.
#[derive(Deserialize)]
struct CallbackQuery {
    code: String,
    state: String,
}

/// Creates the auth router.
pub fn router<P: Platform>() -> Router<AppState<P>> {
    Router::new()
        .route("/signin", get(signin::<P>))
        .route("/callback", get(callback::<P>))
        .route("/signout", get(signout::<P>))
}

/// Redirects the user to GitHub's authorization page.
async fn signin<P: Platform>(
    State(state): State<AppState<P>>,
    jar: CookieJar,
) -> impl IntoResponse {
    let oauth_state = uuid::Uuid::new_v4().to_string();

    let state_cookie = Cookie::build((OAUTH_STATE_COOKIE, oauth_state.clone()))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::minutes(10))
        .build();

    let redirect_uri = state
        .plat
        .redirect_base_url()
        .join("/auth/callback")
        .expect("valid path join");

    let mut authorize_url = state
        .oauth
        .github_base_url
        .join("/login/oauth/authorize")
        .expect("valid path join");
    authorize_url
        .query_pairs_mut()
        .append_pair("client_id", &state.oauth.client_id)
        .append_pair("redirect_uri", redirect_uri.as_str())
        .append_pair("state", &oauth_state);

    (jar.add(state_cookie), Redirect::to(authorize_url.as_str()))
}

/// Handles the OAuth callback from GitHub.
async fn callback<P>(
    State(state): State<AppState<P>>,
    jar: CookieJar,
    Query(query): Query<CallbackQuery>,
) -> impl IntoResponse
where
    P: Platform,
{
    // Verify the OAuth state parameter matches the cookie (CSRF protection).
    let stored_state = jar.get(OAUTH_STATE_COOKIE).map(|c| c.value().to_owned());
    if stored_state.as_deref() != Some(&query.state) {
        error!("OAuth state mismatch");
        return (jar, Redirect::to("/")).into_response();
    }

    let http_client = state.plat.create_http_client();

    // Exchange the authorization code for an access token.
    let access_token =
        match oauth::exchange_code_for_token(&http_client, &state.oauth, &query.code).await {
            Ok(token) => token,
            Err(e) => {
                error!("Failed to exchange code for token: {e:?}");
                return (jar, Redirect::to("/")).into_response();
            }
        };

    // Fetch user info from GitHub.
    let user = match oauth::fetch_user(&http_client, &state.oauth, &access_token).await {
        Ok(user) => user,
        Err(e) => {
            error!("Failed to fetch user info: {e:?}");
            return (jar, Redirect::to("/")).into_response();
        }
    };

    // Create a session.
    let session = Session {
        id: SessionId::new(),
        user,
        access_token,
    };
    let session_id_value = session.id.as_str().to_owned();

    if let Err(e) = state.sessions.put(session) {
        error!("Failed to store session: {e}");
        return (jar, Redirect::to("/")).into_response();
    }

    let session_cookie = Cookie::build((SESSION_COOKIE, session_id_value))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::days(7))
        .build();

    // Clear the OAuth state cookie.
    let remove_state = Cookie::build(OAUTH_STATE_COOKIE)
        .path("/")
        .max_age(time::Duration::ZERO)
        .build();

    let jar = jar.add(session_cookie).add(remove_state);
    (jar, Redirect::to("/dashboard")).into_response()
}

/// Signs the user out by clearing the session.
async fn signout<P: Platform>(
    State(state): State<AppState<P>>,
    jar: CookieJar,
) -> impl IntoResponse {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        let id = SessionId::from(cookie.value().to_owned());
        let _ = state.sessions.delete(&id);
    }

    let remove_cookie = Cookie::build(SESSION_COOKIE)
        .path("/")
        .max_age(time::Duration::ZERO)
        .build();

    (jar.add(remove_cookie), Redirect::to("/"))
}
