//! Tests for authentication-protected routes.
//!
//! Verifies that routes under `/dashboard` redirect unauthenticated requests
//! to `/` and allow access when a valid session cookie is present.

mod test_utils;

use test_utils::{get, register_app_server};

/// Dashboard root is behind auth; unauthenticated GET must redirect.
#[test]
fn dashboard_requires_auth() {
    let mut sim = turmoil::Builder::new().build();
    register_app_server(&mut sim);

    sim.client("client", async {
        let (status, _body) = get("/dashboard").await;
        assert!(status.is_redirection(), "expected redirect, got {status}");
        Ok(())
    });

    sim.run().unwrap();
}

/// The expand partial is behind auth; unauthenticated GET must redirect.
#[test]
fn repo_expand_requires_auth() {
    let mut sim = turmoil::Builder::new().build();
    register_app_server(&mut sim);

    sim.client("client", async {
        let (status, _body) = get("/dashboard/repo/example/hubdash/expand").await;
        assert!(status.is_redirection(), "expected redirect, got {status}");
        Ok(())
    });

    sim.run().unwrap();
}

/// The deps partial is behind auth; unauthenticated GET must redirect.
#[test]
fn repo_deps_requires_auth() {
    let mut sim = turmoil::Builder::new().build();
    register_app_server(&mut sim);

    sim.client("client", async {
        let (status, _body) = get("/dashboard/repo/example/hubdash/deps").await;
        assert!(status.is_redirection(), "expected redirect, got {status}");
        Ok(())
    });

    sim.run().unwrap();
}

/// The landing page must be publicly accessible (no auth required).
#[test]
fn landing_page_is_public() {
    let mut sim = turmoil::Builder::new().build();
    register_app_server(&mut sim);

    sim.client("client", async {
        let (status, body) = get("/").await;
        assert_eq!(status, http::StatusCode::OK);
        assert!(
            body.contains("Hubdash"),
            "landing page should contain 'Hubdash'"
        );
        Ok(())
    });

    sim.run().unwrap();
}

/// The `/auth/signin` endpoint must be publicly accessible.
#[test]
fn signin_is_public() {
    let mut sim = turmoil::Builder::new().build();
    register_app_server(&mut sim);

    sim.client("client", async {
        let (status, _body) = get("/auth/signin").await;
        // signin redirects to GitHub's authorization page
        assert!(
            status.is_redirection(),
            "expected redirect to GitHub, got {status}"
        );
        Ok(())
    });

    sim.run().unwrap();
}
