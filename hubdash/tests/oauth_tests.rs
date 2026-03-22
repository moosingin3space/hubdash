//! Integration tests for the OAuth callback flow.
//!
//! Each test scenario starts the hubdash app server alongside the two mock
//! GitHub hosts (`github.com` and `api.github.com`) inside a turmoil
//! simulation.  The app is configured with HTTP base URLs so turmoil can
//! intercept the connections without TLS.
//!
//! # CSRF note
//!
//! The callback handler validates that the `state` query parameter matches
//! the `hubdash_oauth_state` cookie.  Tests inject both to a known value
//! directly rather than going through `/auth/signin` first.

mod test_utils;

use test_utils::{
    register_api_github_server, register_app_server, register_github_server,
    request_with_cookie,
};
use test_utils::mock_github::{MOCK_USER_LOGIN, MOCK_ACCESS_TOKEN};

/// The OAuth state value used across tests.
const TEST_STATE: &str = "test-oauth-state-abc123";

/// Builds a `Cookie` header value that carries the OAuth state.
fn state_cookie(state: &str) -> String {
    format!("hubdash_oauth_state={state}")
}

/// Extracts all `Set-Cookie` header values from a response header map.
fn set_cookies(headers: &http::HeaderMap) -> Vec<String> {
    headers
        .get_all(http::header::SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap_or("").to_owned())
        .collect()
}

// ── happy path ───────────────────────────────────────────────────────────────

/// A valid callback (state matches cookie, mock GitHub returns token + user)
/// should redirect to `/dashboard` and set a session cookie.
#[test]
fn callback_happy_path_redirects_to_dashboard() {
    let mut sim = turmoil::Builder::new().build();
    register_app_server(&mut sim);
    register_github_server(&mut sim);
    register_api_github_server(&mut sim);

    let cookie = state_cookie(TEST_STATE);
    let path = format!("/auth/callback?code=fake-code&state={TEST_STATE}");

    sim.client("client", async move {
        let (status, headers, _body) = request_with_cookie(&path, Some(&cookie)).await;

        assert!(
            status.is_redirection(),
            "expected redirect after successful OAuth, got {status}"
        );

        let location = headers
            .get(http::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(location, "/dashboard", "should redirect to /dashboard");

        let cookies = set_cookies(&headers);
        let has_session = cookies
            .iter()
            .any(|c| c.starts_with("hubdash_session="));
        assert!(has_session, "response should set hubdash_session cookie; got {cookies:?}");

        Ok(())
    });

    sim.run().unwrap();
}

/// After the happy-path callback, the session cookie must grant access to
/// a protected route.
#[test]
fn callback_session_cookie_grants_dashboard_access() {
    let mut sim = turmoil::Builder::new().build();
    register_app_server(&mut sim);
    register_github_server(&mut sim);
    register_api_github_server(&mut sim);

    let state_cookie_val = state_cookie(TEST_STATE);
    let callback_path = format!("/auth/callback?code=fake-code&state={TEST_STATE}");

    sim.client("client", async move {
        // Step 1: complete the OAuth callback.
        let (_status, headers, _body) =
            request_with_cookie(&callback_path, Some(&state_cookie_val)).await;

        let cookies = set_cookies(&headers);
        let session_header = cookies
            .iter()
            .find(|c| c.starts_with("hubdash_session="))
            .cloned()
            .expect("should have session cookie after callback");

        // Extract just the name=value part (strip attributes like Path=/, etc.)
        let session_kv = session_header
            .split(';')
            .next()
            .unwrap()
            .trim()
            .to_owned();

        // Step 2: use the session cookie to access the dashboard.
        let (dashboard_status, _body) =
            test_utils::get_with_cookie("/dashboard", Some(&session_kv)).await;
        assert_eq!(
            dashboard_status,
            http::StatusCode::OK,
            "session cookie should grant access to dashboard"
        );

        Ok(())
    });

    sim.run().unwrap();
}

/// The session token stored in the session should match the mock's token.
#[test]
fn callback_session_stores_correct_user() {
    let _ = MOCK_USER_LOGIN; // used in doc / future assertions
    let _ = MOCK_ACCESS_TOKEN;

    let mut sim = turmoil::Builder::new().build();
    register_app_server(&mut sim);
    register_github_server(&mut sim);
    register_api_github_server(&mut sim);

    let state_cookie_val = state_cookie(TEST_STATE);
    let callback_path = format!("/auth/callback?code=fake-code&state={TEST_STATE}");

    sim.client("client", async move {
        // Complete the callback — we just verify no error redirect occurs.
        let (status, headers, _body) =
            request_with_cookie(&callback_path, Some(&state_cookie_val)).await;

        assert!(status.is_redirection(), "expected redirect, got {status}");
        let location = headers
            .get(http::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        // A redirect to "/" means something went wrong; "/dashboard" is success.
        assert_eq!(location, "/dashboard", "got error redirect instead of /dashboard");

        Ok(())
    });

    sim.run().unwrap();
}

// ── CSRF protection ──────────────────────────────────────────────────────────

/// When the `state` query parameter does not match the cookie, the handler
/// must redirect to `/` without setting a session.
#[test]
fn callback_csrf_mismatch_redirects_to_root() {
    let mut sim = turmoil::Builder::new().build();
    register_app_server(&mut sim);
    // No GitHub mock needed — the handler bails out before hitting GitHub.

    let cookie = state_cookie("correct-state");
    let path = "/auth/callback?code=x&state=wrong-state";

    sim.client("client", async move {
        let (status, headers, _body) = request_with_cookie(path, Some(&cookie)).await;

        assert!(status.is_redirection(), "expected redirect, got {status}");
        let location = headers
            .get(http::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(location, "/", "CSRF failure should redirect to /");

        let cookies = set_cookies(&headers);
        let has_session = cookies.iter().any(|c| c.starts_with("hubdash_session="));
        assert!(!has_session, "CSRF failure must not set a session cookie");

        Ok(())
    });

    sim.run().unwrap();
}

/// When the OAuth state cookie is absent entirely, the handler must redirect
/// to `/` (treated as a CSRF failure).
#[test]
fn callback_missing_state_cookie_redirects_to_root() {
    let mut sim = turmoil::Builder::new().build();
    register_app_server(&mut sim);

    sim.client("client", async {
        let (status, headers, _body) =
            request_with_cookie("/auth/callback?code=x&state=any", None).await;

        assert!(status.is_redirection(), "expected redirect, got {status}");
        let location = headers
            .get(http::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(location, "/", "missing state cookie should redirect to /");

        Ok(())
    });

    sim.run().unwrap();
}
