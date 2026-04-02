//! Tests for authentication-protected routes.
//!
//! Verifies that routes under `/dashboard` redirect unauthenticated requests
//! to `/` and allow access when a valid session cookie is present.

use super::test_utils::{get, get_with_cookie, run_authed_sim, run_sim};

/// Dashboard root is behind auth; unauthenticated GET must redirect.
#[test]
fn dashboard_requires_auth() {
    run_sim(|| async {
        let (status, _body) = get("/dashboard").await;
        assert!(status.is_redirection(), "expected redirect, got {status}");
        Ok(())
    });
}

/// The expand partial is behind auth; unauthenticated GET must redirect.
#[test]
fn repo_expand_requires_auth() {
    run_sim(|| async {
        let (status, _body) = get("/dashboard/repo/example/hubdash/expand").await;
        assert!(status.is_redirection(), "expected redirect, got {status}");
        Ok(())
    });
}

/// The landing page must be publicly accessible (no auth required).
#[test]
fn landing_page_is_public() {
    run_sim(|| async {
        let (status, body) = get("/").await;
        assert_eq!(status, http::StatusCode::OK);
        assert!(
            body.contains("Hubdash"),
            "landing page should contain 'Hubdash'"
        );
        Ok(())
    });
}

/// Root route redirects to `/dashboard` when a valid session cookie is present.
#[test]
fn root_redirects_to_dashboard_when_authenticated() {
    run_authed_sim(|cookie| async move {
        let (status, _body) = get_with_cookie("/", Some(&cookie)).await;
        assert!(status.is_redirection(), "expected redirect, got {status}");
        Ok(())
    });
}

/// The `/auth/signin` endpoint must be publicly accessible.
#[test]
fn signin_is_public() {
    run_sim(|| async {
        let (status, _body) = get("/auth/signin").await;
        // signin redirects to GitHub's authorization page
        assert!(
            status.is_redirection(),
            "expected redirect to GitHub, got {status}"
        );
        Ok(())
    });
}
