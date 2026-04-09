//! Tests for dashboard partials and the dashboard page.
//!
//! Each test pre-seeds a valid session via [`run_authed_sim`], which passes
//! the ready-to-use `Cookie` header value to the client closure.

use super::test_utils::{get_with_cookie, run_authed_sim};

// ── repo_expand ──────────────────────────────────────────────────────────────

/// `repo_expand` for a known repo returns detail HTML with pipeline info.
#[test]
fn repo_expand_returns_detail_html() {
    run_authed_sim(|cookie| async move {
        let (status, body) =
            get_with_cookie("/dashboard/repo/example/hubdash/expand", Some(&cookie)).await;
        assert_eq!(status, http::StatusCode::OK, "body: {body}");
        assert!(
            body.contains("repo-detail"),
            "expected repo-detail markup, got: {body}"
        );
        assert!(
            body.contains("repo-github-link"),
            "expected github link, got: {body}"
        );
        Ok(())
    });
}

/// `repo_expand` for an unknown repo returns the detail shell (pipelines load lazily).
#[test]
fn repo_expand_unknown_repo() {
    run_authed_sim(|cookie| async move {
        let (status, body) =
            get_with_cookie("/dashboard/repo/nobody/norepo/expand", Some(&cookie)).await;
        assert_eq!(status, http::StatusCode::OK, "body: {body}");
        assert!(
            body.contains("repo-detail"),
            "expected repo-detail markup, got: {body}"
        );
        Ok(())
    });
}

// ── dashboard page ───────────────────────────────────────────────────────────

/// The dashboard page lists all mock repositories.
#[test]
fn dashboard_page_lists_repos() {
    run_authed_sim(|cookie| async move {
        let (status, body) = get_with_cookie("/dashboard", Some(&cookie)).await;
        assert_eq!(status, http::StatusCode::OK, "body: {body}");
        assert!(
            body.contains("repo-table"),
            "expected repo-table, got: {body}"
        );
        assert!(body.contains("hubdash"), "expected hubdash repo row");
        assert!(
            body.contains("api-gateway"),
            "expected api-gateway repo row"
        );
        assert!(
            body.contains("frontend-app"),
            "expected frontend-app repo row"
        );
        Ok(())
    });
}
